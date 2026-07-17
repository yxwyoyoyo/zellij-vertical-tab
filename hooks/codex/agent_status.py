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


PIPE_NAME = "vertical-tab-agent-status"
PROTOCOL_VERSION = 1
EVENT_STATES = {
    "SessionStart": "idle",
    "UserPromptSubmit": "working",
    "PreToolUse": "working",
    "PermissionRequest": "waiting",
    "Stop": "done",
}


def build_payload(hook_input: dict[str, Any], pane_id: str) -> dict[str, Any] | None:
    state = EVENT_STATES.get(hook_input.get("hook_event_name"))
    session_id = hook_input.get("session_id")
    if state is None or not isinstance(session_id, str) or not session_id.strip():
        return None
    return {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": state,
        "updated_at_ms": time.time_ns() // 1_000_000,
    }


def build_clear_payload(pane_id: str, session_id: str) -> dict[str, Any]:
    return {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": "clear",
        "updated_at_ms": time.time_ns() // 1_000_000,
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


def watch_process(pid: int, pane_id: str, session_id: str) -> None:
    while process_is_running(pid):
        time.sleep(0.5)
    publish_payload(build_clear_payload(pane_id, session_id))


def start_exit_watcher(pid: int, pane_id: str, session_id: str) -> None:
    try:
        subprocess.Popen(
            [
                sys.executable,
                str(Path(__file__).resolve()),
                "--watch",
                str(pid),
                pane_id,
                session_id,
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
            close_fds=True,
        )
    except OSError:
        pass


def main() -> int:
    if len(sys.argv) == 5 and sys.argv[1] == "--watch":
        try:
            watched_pid = int(sys.argv[2])
        except ValueError:
            return 0
        watch_process(watched_pid, sys.argv[3], sys.argv[4])
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
    publish_payload(payload)
    if hook_input.get("hook_event_name") == "SessionStart":
        codex_pid = find_codex_ancestor()
        if codex_pid is not None:
            start_exit_watcher(codex_pid, pane_id, payload["session_id"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
