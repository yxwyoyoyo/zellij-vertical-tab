#!/usr/bin/env python3
"""Publish Claude Code lifecycle state to the zellij-vertical-tab plugin.

Claude Code sends one JSON object on stdin for each configured command hook.
The bridge is deliberately best-effort: missing environment, malformed input,
storage errors, and Zellij errors must never block or alter an agent turn.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import sys
from typing import Any

COMMON_DIRECTORY = Path(__file__).resolve().parent.parent / "common"
if COMMON_DIRECTORY.is_dir():
    sys.path.insert(0, str(COMMON_DIRECTORY))

try:
    from agent_bridge import AgentUpdate
    from agent_bridge import dispatch_update
    from agent_bridge import snapshot_for_argument
except ImportError:
    AgentUpdate = None

    def dispatch_update(*_args: object, **_kwargs: object) -> None:
        return None

    def snapshot_for_argument(_value: object) -> None:
        return None


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


def build_update(hook_input: dict[str, Any]) -> Any | None:
    hook_event = hook_input.get("hook_event_name")
    session_id = hook_input.get("session_id")
    if (
        AgentUpdate is None
        or not isinstance(session_id, str)
        or not session_id.strip()
    ):
        return None
    if hook_event == "SessionEnd":
        return AgentUpdate(
            session_id=session_id,
            state="clear",
            event="session_exit",
        )
    state = EVENT_STATES.get(hook_event)
    if state is None:
        return None
    prompt_id = hook_input.get("prompt_id")
    return AgentUpdate(
        session_id=session_id,
        state=state,
        event=EVENT_NAMES[hook_event],
        turn_id=(
            prompt_id
            if isinstance(prompt_id, str) and prompt_id.strip()
            else None
        ),
    )


def process_hook(hook_input: object, pane_id: str) -> bool:
    if not isinstance(hook_input, dict):
        return False
    update = build_update(hook_input)
    if update is None:
        return False
    return dispatch_update(
        update,
        pane_id,
        prune_dead=hook_input.get("hook_event_name") == "SessionStart",
    ) is not None


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
        snapshot = snapshot_for_argument(sys.argv[2])
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
