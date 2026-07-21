#!/usr/bin/env python3
"""Publish Codex turn-complete notifications and preserve an existing notifier."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Any

COMMON_DIRECTORY = Path(__file__).resolve().parent.parent / "common"
if COMMON_DIRECTORY.is_dir():
    sys.path.insert(0, str(COMMON_DIRECTORY))

try:
    from agent_bridge import AgentUpdate
    from agent_bridge import dispatch_update
except ImportError:
    AgentUpdate = None

    def dispatch_update(*_args: object, **_kwargs: object) -> None:
        return None


def split_arguments(arguments: list[str]) -> tuple[list[str], str] | None:
    if not arguments:
        return None
    if arguments[0] != "--forward":
        return [], arguments[-1]
    try:
        separator = arguments.index("--", 1)
    except ValueError:
        return None
    if separator == 1 or separator + 1 != len(arguments) - 1:
        return None
    return arguments[1:separator], arguments[-1]


def build_done_update(raw_notification: str) -> Any | None:
    try:
        notification = json.loads(raw_notification)
    except json.JSONDecodeError:
        return None
    if not isinstance(notification, dict):
        return None
    if AgentUpdate is None or notification.get("type") != "agent-turn-complete":
        return None
    session_id = notification.get("thread-id", notification.get("thread_id"))
    if not isinstance(session_id, str) or not session_id.strip():
        return None
    turn_id = notification.get("turn-id", notification.get("turn_id"))
    return AgentUpdate(
        session_id=session_id,
        state="done",
        event="agent_turn_complete",
        turn_id=turn_id if isinstance(turn_id, str) and turn_id.strip() else None,
    )


def forward_notification(command: list[str], raw_notification: str) -> None:
    if not command:
        return
    try:
        subprocess.run(
            [*command, raw_notification],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        pass


def main() -> int:
    parsed = split_arguments(sys.argv[1:])
    if parsed is None:
        return 0
    forward_command, raw_notification = parsed
    update = build_done_update(raw_notification)
    if update is not None:
        dispatch_update(update)
    forward_notification(forward_command, raw_notification)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
