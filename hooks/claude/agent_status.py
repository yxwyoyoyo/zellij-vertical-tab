#!/usr/bin/env python3
"""Publish Claude Code lifecycle state to the zellij-vertical-tab plugin.

Claude Code sends one JSON object on stdin for each configured command hook.
The bridge is deliberately best-effort: missing environment, malformed input,
storage errors, and Zellij errors must never block or alter an agent turn.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from typing import Any

try:
    from status_store import apply_payload as persist_payload
    from status_store import find_zellij_ancestor
    from status_store import parse_positive_pid
    from status_store import prune_dead_server_directories
    from status_store import snapshot_json
except ImportError:
    def persist_payload(_payload: object, _zellij_pid: int) -> bool:
        return False

    def find_zellij_ancestor(_start_pid: int | None = None) -> int | None:
        return None

    def parse_positive_pid(_value: object) -> int | None:
        return None

    def prune_dead_server_directories(_current_zellij_pid: int) -> int:
        return 0

    def snapshot_json(_zellij_pid: int) -> str | None:
        return None


PIPE_NAME = "vertical-tab-agent-status"
PROTOCOL_VERSION = 1
BEL = "\a"
EVENT_STATES = {
    "SessionStart": "idle",
    "UserPromptSubmit": "working",
    "PreToolUse": "working",
    "PermissionRequest": "waiting",
    "PostToolUse": "working",
    "PostToolUseFailure": "working",
    "PermissionDenied": "working",
    "Stop": "done",
}
EVENT_NAMES = {
    "SessionStart": "session_start",
    "UserPromptSubmit": "user_prompt_submit",
    "PreToolUse": "pre_tool_use",
    "PermissionRequest": "permission_request",
    "PostToolUse": "post_tool_use",
    "PostToolUseFailure": "post_tool_use_failure",
    "PermissionDenied": "permission_denied",
    "Stop": "stop",
}


def build_payload(hook_input: dict[str, Any], pane_id: str) -> dict[str, Any] | None:
    hook_event = hook_input.get("hook_event_name")
    session_id = hook_input.get("session_id")
    if not isinstance(session_id, str) or not session_id.strip():
        return None
    if hook_event == "SessionEnd":
        return build_clear_payload(pane_id, session_id)
    state = EVENT_STATES.get(hook_event)
    if state is None:
        return None
    payload = {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": state,
        "updated_at_ms": time.time_ns() // 1_000_000,
        "event": EVENT_NAMES[hook_event],
    }
    prompt_id = hook_input.get("prompt_id")
    if isinstance(prompt_id, str) and prompt_id.strip():
        payload["turn_id"] = prompt_id
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


def process_hook(hook_input: object, pane_id: str) -> bool:
    if not isinstance(hook_input, dict):
        return False
    payload = build_payload(hook_input, pane_id)
    if payload is None:
        return False
    zellij_pid = find_zellij_ancestor()
    if hook_input.get("hook_event_name") == "SessionStart" and zellij_pid is not None:
        prune_dead_server_directories(zellij_pid)
    if zellij_pid is not None:
        persist_payload(payload, zellij_pid)
    publish_payload(payload)
    return True


def notification_output(hook_input: object) -> dict[str, str] | None:
    """Return Claude's terminal-sequence response for user-attention events."""
    if not isinstance(hook_input, dict):
        return None
    hook_event = hook_input.get("hook_event_name")
    if hook_event == "PermissionRequest":
        return {"terminalSequence": BEL}
    if hook_event == "Stop" and hook_input.get("stop_hook_active") is not True:
        return {"terminalSequence": BEL}
    return None


def main() -> int:
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
    process_hook(hook_input, pane_id)
    output = notification_output(hook_input)
    if output is not None:
        print(json.dumps(output, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
