#!/usr/bin/env python3
"""Publish Codex lifecycle state to the zellij-vertical-tab plugin.

Codex sends one JSON object on stdin for each configured hook. This bridge is
deliberately best-effort: missing environment, malformed input, and Zellij
errors must never block or otherwise change an agent turn.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import Any

try:
    from status_store import apply_payload as persist_payload
    from status_store import find_zellij_ancestor
    from status_store import parse_positive_pid
    from status_store import snapshot_json
except ImportError:
    def persist_payload(_payload: object, _zellij_pid: int) -> bool:
        return False

    def find_zellij_ancestor(_start_pid: int | None = None) -> int | None:
        return None

    def parse_positive_pid(_value: object) -> int | None:
        return None

    def snapshot_json(_zellij_pid: int) -> str | None:
        return None


PIPE_NAME = "vertical-tab-agent-status"
PROTOCOL_VERSION = 1
EVENT_STATES = {
    "SessionStart": "idle",
    "UserPromptSubmit": "working",
    "PreToolUse": "working",
    "PermissionRequest": "waiting",
    "PostToolUse": "working",
    "Stop": "done",
}
EVENT_NAMES = {
    "SessionStart": "session_start",
    "UserPromptSubmit": "user_prompt_submit",
    "PreToolUse": "pre_tool_use",
    "PermissionRequest": "permission_request",
    "PostToolUse": "post_tool_use",
    "Stop": "stop",
}
TRANSCRIPT_TAIL_MAX_BYTES = 8 * 1024 * 1024


def approvals_reviewer_for_turn(
    transcript_path: object, turn_id: object
) -> str | None:
    if not isinstance(transcript_path, str) or not transcript_path.strip():
        return None
    if not isinstance(turn_id, str) or not turn_id.strip():
        return None
    try:
        with open(transcript_path, "rb") as transcript:
            transcript.seek(0, os.SEEK_END)
            size = transcript.tell()
            start = max(0, size - TRANSCRIPT_TAIL_MAX_BYTES)
            transcript.seek(start)
            if start:
                transcript.readline()
            reviewer = None
            for raw_line in transcript:
                try:
                    record = json.loads(raw_line)
                except (json.JSONDecodeError, UnicodeDecodeError):
                    continue
                if not isinstance(record, dict) or record.get("type") != "turn_context":
                    continue
                payload = record.get("payload")
                if not isinstance(payload, dict) or payload.get("turn_id") != turn_id:
                    continue
                candidate = payload.get("approvals_reviewer")
                if isinstance(candidate, str):
                    reviewer = candidate
            return reviewer
    except OSError:
        return None


def build_payload(hook_input: dict[str, Any], pane_id: str) -> dict[str, Any] | None:
    hook_event = hook_input.get("hook_event_name")
    state = EVENT_STATES.get(hook_event)
    session_id = hook_input.get("session_id")
    if state is None or not isinstance(session_id, str) or not session_id.strip():
        return None
    turn_id = hook_input.get("turn_id")
    if hook_event == "PermissionRequest" and approvals_reviewer_for_turn(
        hook_input.get("transcript_path"), turn_id
    ) == "auto_review":
        state = "working"
    payload = {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": state,
        "updated_at_ms": time.time_ns() // 1_000_000,
        "event": EVENT_NAMES[hook_event],
    }
    if isinstance(turn_id, str) and turn_id.strip():
        payload["turn_id"] = turn_id
    return payload


def build_clear_payload(pane_id: str, session_id: str) -> dict[str, Any]:
    return {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": "clear",
        "updated_at_ms": time.time_ns() // 1_000_000,
        "event": "session_exit",
    }


def publish_payload(payload: dict[str, Any]) -> None:
    try:
        subprocess.run(
            [
                "zellij",
                "pipe",
                "--name",
                PIPE_NAME,
                "--",
                json.dumps(payload, separators=(",", ":")),
            ],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=2,
        )
    except (OSError, subprocess.SubprocessError):
        pass


def process_info(pid: int) -> tuple[int, str] | None:
    try:
        result = subprocess.run(
            ["ps", "-o", "ppid=", "-o", "comm=", "-p", str(pid)],
            check=False,
            capture_output=True,
            text=True,
            timeout=1,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    fields = result.stdout.strip().split(maxsplit=1)
    if result.returncode != 0 or len(fields) != 2:
        return None
    try:
        return int(fields[0]), fields[1]
    except ValueError:
        return None


def find_codex_ancestor(start_pid: int | None = None) -> int | None:
    pid = os.getppid() if start_pid is None else start_pid
    for _ in range(12):
        info = process_info(pid)
        if info is None:
            return None
        parent_pid, command = info
        if Path(command).name.lower() in {"codex", "codex.exe"}:
            return pid
        if parent_pid <= 1 or parent_pid == pid:
            return None
        pid = parent_pid
    return None


def process_is_running(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except PermissionError:
        return True
    except ProcessLookupError:
        return False


def watch_process(
    pid: int, pane_id: str, session_id: str, zellij_pid: int | None = None
) -> None:
    while process_is_running(pid):
        time.sleep(0.5)
    payload = build_clear_payload(pane_id, session_id)
    if zellij_pid is not None:
        persist_payload(payload, zellij_pid)
    publish_payload(payload)


def start_exit_watcher(
    pid: int, pane_id: str, session_id: str, zellij_pid: int | None = None
) -> None:
    arguments = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--watch",
        str(pid),
        pane_id,
        session_id,
    ]
    if zellij_pid is not None:
        arguments.append(str(zellij_pid))
    try:
        subprocess.Popen(
            arguments,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
            close_fds=True,
        )
    except OSError:
        pass


def main() -> int:
    if len(sys.argv) in {5, 6} and sys.argv[1] == "--watch":
        try:
            watched_pid = int(sys.argv[2])
        except ValueError:
            return 0
        zellij_pid = parse_positive_pid(sys.argv[5]) if len(sys.argv) == 6 else None
        watch_process(watched_pid, sys.argv[3], sys.argv[4], zellij_pid)
        return 0

    if len(sys.argv) == 3 and sys.argv[1] == "--snapshot":
        zellij_pid = parse_positive_pid(sys.argv[2])
        if zellij_pid is not None:
            snapshot = snapshot_json(zellij_pid)
            if snapshot is not None:
                print(snapshot)
        return 0

    pane_id = os.environ.get("ZELLIJ_PANE_ID", "").strip()
    if not pane_id:
        return 0

    try:
        hook_input = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError):
        return 0
    if not isinstance(hook_input, dict):
        return 0

    payload = build_payload(hook_input, pane_id)
    if payload is None:
        return 0
    zellij_pid = find_zellij_ancestor()
    if zellij_pid is not None:
        persist_payload(payload, zellij_pid)
    publish_payload(payload)
    if hook_input.get("hook_event_name") == "SessionStart":
        codex_pid = find_codex_ancestor()
        if codex_pid is not None:
            start_exit_watcher(codex_pid, pane_id, payload["session_id"], zellij_pid)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
