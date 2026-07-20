#!/usr/bin/env python3
"""Publish Codex turn-complete notifications and preserve an existing notifier."""

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
except ImportError:
    def persist_payload(_payload: object, _zellij_pid: int) -> bool:
        return False

    def find_zellij_ancestor(_start_pid: int | None = None) -> int | None:
        return None


PIPE_NAME = "vertical-tab-agent-status"
PROTOCOL_VERSION = 1


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


def build_done_payload(raw_notification: str, pane_id: str) -> dict[str, Any] | None:
    try:
        notification = json.loads(raw_notification)
    except json.JSONDecodeError:
        return None
    if not isinstance(notification, dict):
        return None
    if notification.get("type") != "agent-turn-complete":
        return None
    session_id = notification.get("thread-id", notification.get("thread_id"))
    if not isinstance(session_id, str) or not session_id.strip():
        return None
    payload = {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": "done",
        "updated_at_ms": time.time_ns() // 1_000_000,
        "event": "agent_turn_complete",
    }
    turn_id = notification.get("turn-id", notification.get("turn_id"))
    if isinstance(turn_id, str) and turn_id.strip():
        payload["turn_id"] = turn_id
    return payload


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
    pane_id = os.environ.get("ZELLIJ_PANE_ID", "").strip()
    if pane_id:
        payload = build_done_payload(raw_notification, pane_id)
        if payload is not None:
            zellij_pid = find_zellij_ancestor()
            if zellij_pid is not None:
                persist_payload(payload, zellij_pid)
            publish_payload(payload)
    forward_notification(forward_command, raw_notification)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
