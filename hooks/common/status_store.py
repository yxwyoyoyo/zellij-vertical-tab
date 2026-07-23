#!/usr/bin/env python3
"""Durable, dependency-free lifecycle journal shared by agent adapters."""

from __future__ import annotations

import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from typing import Any


PROTOCOL_VERSION = 1
SUPPORTED_STATES = {"idle", "working", "waiting", "done", "clear"}
SUPPORTED_EVENTS = {
    "session_start",
    "user_prompt_submit",
    "pre_tool_use",
    "permission_request",
    "post_tool_use",
    "post_tool_use_failure",
    "permission_denied",
    "stop",
    "agent_turn_complete",
    "session_exit",
}
MAX_RECORD_BYTES = 64 * 1024
MAX_ANCESTORS = 16
STALE_RECORD_GRACE_MS = 6 * 60 * 60 * 1000  # 6 hours — crash cleanup safety margin
STATE_DIR_ENV = "ZELLIJ_VERTICAL_TAB_STATE_DIR"


def parse_positive_pid(value: object) -> int | None:
    if isinstance(value, bool):
        return None
    try:
        pid = int(value)
    except (TypeError, ValueError):
        return None
    return pid if pid > 0 else None


def parse_pane_id(value: object) -> int | None:
    if not isinstance(value, str):
        return None
    digits = value.removeprefix("terminal_")
    if not digits or not digits.isascii() or not digits.isdigit():
        return None
    return int(digits)


def validate_payload(payload: object) -> dict[str, Any] | None:
    if not isinstance(payload, dict):
        return None
    pane_id = parse_pane_id(payload.get("pane_id"))
    session_id = payload.get("session_id")
    state = payload.get("state")
    updated_at_ms = payload.get("updated_at_ms")
    event = payload.get("event")
    turn_id = payload.get("turn_id")
    if (
        payload.get("version") != PROTOCOL_VERSION
        or pane_id is None
        or not isinstance(session_id, str)
        or not session_id.strip()
        or state not in SUPPORTED_STATES
        or isinstance(updated_at_ms, bool)
        or not isinstance(updated_at_ms, int)
        or updated_at_ms <= 0
        or (event is not None and event not in SUPPORTED_EVENTS)
        or (turn_id is not None and (not isinstance(turn_id, str) or not turn_id.strip()))
    ):
        return None
    validated = {
        "version": PROTOCOL_VERSION,
        "pane_id": f"terminal_{pane_id}",
        "session_id": session_id,
        "state": state,
        "updated_at_ms": updated_at_ms,
    }
    if event is not None:
        validated["event"] = event
    if turn_id is not None:
        validated["turn_id"] = turn_id
    return validated


def journal_root() -> Path:
    override = os.environ.get(STATE_DIR_ENV)
    if override:
        return Path(override)
    cache_home = os.environ.get("XDG_CACHE_HOME")
    return (
        Path(cache_home) / "zellij-vertical-tab"
        if cache_home
        else Path.home() / ".cache" / "zellij-vertical-tab"
    )


def server_directory(zellij_pid: int) -> Path | None:
    parsed_pid = parse_positive_pid(zellij_pid)
    return journal_root() / "sessions" / str(parsed_pid) if parsed_pid else None


def process_is_running(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except PermissionError:
        return True
    except ProcessLookupError:
        return False


def prune_dead_server_directories(current_zellij_pid: int) -> int:
    current_pid = parse_positive_pid(current_zellij_pid)
    if current_pid is None:
        return 0
    sessions = journal_root() / "sessions"
    try:
        entries = list(sessions.iterdir())
    except OSError:
        return 0
    removed = 0
    for entry in entries:
        stale_pid = parse_positive_pid(entry.name)
        if (
            stale_pid is None
            or stale_pid == current_pid
            or not entry.is_dir()
            or process_is_running(stale_pid)
        ):
            continue
        try:
            shutil.rmtree(entry)
            removed += 1
        except OSError:
            continue
    return removed


def _file_for_pane(directory: Path, pane_id: int) -> Path:
    return directory / f"terminal_{pane_id}.json"


def _lock_for_pane(directory: Path, pane_id: int) -> Path:
    return directory / f"terminal_{pane_id}.lock"


def prune_stale_pane_records(zellij_pid: int, keep_session_id: str | None = None) -> int:
    """Remove pane records that are explicitly expired or from long-dead sessions.

    Safe to call frequently — only ``"clear"`` state records (explicitly
    expired via ``SessionEnd``) and records older than the grace period with a
    different *session_id* are removed.
    """
    directory = server_directory(zellij_pid)
    if directory is None or not directory.is_dir():
        return 0
    now_ms = time.time_ns() // 1_000_000
    removed = 0
    try:
        paths = list(directory.glob("terminal_*.json"))
    except OSError:
        return 0
    for path in paths:
        pane_id = parse_pane_id(path.stem)
        record = _read_record(path)
        if pane_id is None or record is None:
            continue
        record_session = record.get("session_id")
        record_state = record.get("state")
        record_age = now_ms - record["updated_at_ms"]

        # Explicitly expired: SessionEnd fired, state is "clear".
        if record_state == "clear":
            should_remove = True
        # Same session — keep (it's the current session, or a concurrent one).
        elif (
            isinstance(keep_session_id, str)
            and isinstance(record_session, str)
            and record_session == keep_session_id
        ):
            continue
        # Stale record from a different session — likely a crash zombie.
        # Only age-prune when we have a keep-session to anchor against.
        elif (
            isinstance(keep_session_id, str)
            and record_age > STALE_RECORD_GRACE_MS
        ):
            should_remove = True
        else:
            continue

        lock_path = _lock_for_pane(directory, pane_id)
        try:
            with lock_path.open("a+b") as lock_file:
                os.chmod(lock_path, 0o600)
                fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
                # Re-read under lock to avoid TOCTOU
                current = _read_record(path)
                if current is None:
                    continue
                current_state = current.get("state")
                if should_remove or current_state == "clear":
                    path.unlink(missing_ok=True)
                    lock_path.unlink(missing_ok=True)
                    removed += 1
        except OSError:
            continue
    return removed


def _read_record(path: Path) -> dict[str, Any] | None:
    try:
        if path.stat().st_size > MAX_RECORD_BYTES:
            return None
        return validate_payload(json.loads(path.read_text(encoding="utf-8")))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None


def _should_apply(current: dict[str, Any] | None, update: dict[str, Any]) -> bool:
    if current is None:
        return True
    if update["updated_at_ms"] < current["updated_at_ms"]:
        return False
    if update["state"] == "clear" and update["session_id"] != current["session_id"]:
        return False
    if (
        current["state"] == "done"
        and update["state"] != "clear"
        and update["session_id"] == current["session_id"]
    ):
        current_turn = current.get("turn_id")
        update_turn = update.get("turn_id")
        if current_turn is not None and update_turn is not None:
            return current_turn != update_turn
        return update.get("event") == "user_prompt_submit"
    return True


def apply_payload(payload: object, zellij_pid: int) -> bool:
    update = validate_payload(payload)
    directory = server_directory(zellij_pid)
    if update is None or directory is None:
        return False
    pane_id = parse_pane_id(update["pane_id"])
    assert pane_id is not None
    try:
        directory.mkdir(mode=0o700, parents=True, exist_ok=True)
        os.chmod(directory, 0o700)
        record_path = directory / f"terminal_{pane_id}.json"
        lock_path = directory / f"terminal_{pane_id}.lock"
        with lock_path.open("a+b") as lock_file:
            os.chmod(lock_path, 0o600)
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
            if not _should_apply(_read_record(record_path), update):
                return False
            temporary_path: Path | None = None
            try:
                with tempfile.NamedTemporaryFile(
                    mode="w",
                    encoding="utf-8",
                    dir=directory,
                    prefix=f".terminal_{pane_id}.",
                    suffix=".tmp",
                    delete=False,
                ) as temporary:
                    temporary_path = Path(temporary.name)
                    os.fchmod(temporary.fileno(), 0o600)
                    json.dump(update, temporary, separators=(",", ":"))
                    temporary.write("\n")
                    temporary.flush()
                    os.fsync(temporary.fileno())
                os.replace(temporary_path, record_path)
                temporary_path = None
            finally:
                if temporary_path is not None:
                    temporary_path.unlink(missing_ok=True)
        return True
    except OSError:
        return False


def load_snapshot(zellij_pid: int) -> dict[str, Any] | None:
    directory = server_directory(zellij_pid)
    if directory is None:
        return None
    records: list[dict[str, Any]] = []
    try:
        paths = list(directory.glob("terminal_*.json"))
    except OSError:
        paths = []
    for path in paths:
        pane_id = parse_pane_id(path.stem)
        record = _read_record(path)
        if pane_id is None or record is None or parse_pane_id(record["pane_id"]) != pane_id:
            continue
        records.append(record)
    records.sort(key=lambda record: parse_pane_id(record["pane_id"]) or 0)
    return {
        "version": PROTOCOL_VERSION,
        "records": records,
        "acknowledgements": [],
    }


def snapshot_json(zellij_pid: int) -> str | None:
    snapshot = load_snapshot(zellij_pid)
    return json.dumps(snapshot, separators=(",", ":")) if snapshot is not None else None


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


def find_zellij_ancestor(start_pid: int | None = None) -> int | None:
    pid = os.getppid() if start_pid is None else start_pid
    for _ in range(MAX_ANCESTORS):
        info = process_info(pid)
        if info is None:
            return None
        parent_pid, command = info
        if Path(command).name.lower() in {"zellij", "zellij.exe"}:
            return pid
        if parent_pid <= 1 or parent_pid == pid:
            return None
        pid = parent_pid
    return None
