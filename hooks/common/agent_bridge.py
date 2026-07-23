#!/usr/bin/env python3
"""Agent-neutral lifecycle transport for zellij-vertical-tab adapters."""

from __future__ import annotations

from dataclasses import dataclass
import json
import os
import subprocess
import time
from typing import Any

from status_store import apply_payload as persist_payload
from status_store import find_zellij_ancestor
from status_store import parse_positive_pid
from status_store import prune_dead_server_directories
from status_store import prune_stale_pane_records
from status_store import snapshot_json
from status_store import validate_payload


PIPE_NAME = "vertical-tab-agent-status"
PROTOCOL_VERSION = 1


@dataclass(frozen=True)
class AgentUpdate:
    """Canonical lifecycle update produced by an agent-specific adapter."""

    session_id: str
    state: str
    event: str
    turn_id: str | None = None


def build_payload(
    update: AgentUpdate,
    pane_id: str,
    updated_at_ms: int | None = None,
) -> dict[str, Any] | None:
    payload: dict[str, Any] = {
        "version": PROTOCOL_VERSION,
        "pane_id": pane_id,
        "session_id": update.session_id,
        "state": update.state,
        "updated_at_ms": (
            time.time_ns() // 1_000_000
            if updated_at_ms is None
            else updated_at_ms
        ),
        "event": update.event,
    }
    if update.turn_id is not None:
        payload["turn_id"] = update.turn_id
    return payload if validate_payload(payload) is not None else None


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


def dispatch_update(
    update: AgentUpdate,
    pane_id: str | None = None,
    zellij_pid: int | None = None,
    *,
    discover_zellij: bool = True,
    prune_dead: bool = False,
) -> dict[str, Any] | None:
    owning_pane = (
        os.environ.get("ZELLIJ_PANE_ID", "").strip()
        if pane_id is None
        else pane_id.strip()
    )
    if not owning_pane:
        return None
    payload = build_payload(update, owning_pane)
    if payload is None:
        return None
    owning_zellij = (
        find_zellij_ancestor()
        if zellij_pid is None and discover_zellij
        else zellij_pid
    )
    if prune_dead and owning_zellij is not None:
        prune_dead_server_directories(owning_zellij)
        prune_stale_pane_records(owning_zellij, keep_session_id=update.session_id)
    if owning_zellij is not None:
        persist_payload(payload, owning_zellij)
    publish_payload(payload)
    return payload


def snapshot_for_argument(value: object) -> str | None:
    zellij_pid = parse_positive_pid(value)
    return snapshot_json(zellij_pid) if zellij_pid is not None else None
