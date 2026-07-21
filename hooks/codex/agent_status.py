#!/usr/bin/env python3
"""Publish Codex lifecycle state to the zellij-vertical-tab plugin.

Codex sends one JSON object on stdin for each configured hook. This bridge is
deliberately best-effort: missing environment, malformed input, and Zellij
errors must never block or otherwise change an agent turn.
"""

from __future__ import annotations

import errno
import fcntl
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time
from typing import Any, BinaryIO

COMMON_DIRECTORY = Path(__file__).resolve().parent.parent / "common"
if COMMON_DIRECTORY.is_dir():
    sys.path.insert(0, str(COMMON_DIRECTORY))

try:
    from agent_bridge import AgentUpdate
    from agent_bridge import dispatch_update
    from agent_bridge import snapshot_for_argument
    from status_store import journal_root
    from status_store import parse_positive_pid
    from status_store import process_is_running
    from status_store import server_directory
except ImportError:
    AgentUpdate = None

    def dispatch_update(*_args: object, **_kwargs: object) -> None:
        return None

    def snapshot_for_argument(_value: object) -> None:
        return None

    def parse_positive_pid(_value: object) -> int | None:
        return None

    def process_is_running(_pid: int) -> bool:
        return False

    def journal_root() -> Path:
        return Path(tempfile.gettempdir()) / "zellij-vertical-tab"

    def server_directory(_zellij_pid: int) -> Path | None:
        return None

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
PROCESS_ANCESTOR_LIMIT = 16
WATCHER_POLL_INTERVAL_SECONDS = 2.0
WATCHER_DIRECTORY = "watchers"


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
                    return candidate
            return None
    except OSError:
        return None


def build_update(hook_input: dict[str, Any]) -> Any | None:
    hook_event = hook_input.get("hook_event_name")
    state = EVENT_STATES.get(hook_event)
    session_id = hook_input.get("session_id")
    if (
        AgentUpdate is None
        or state is None
        or not isinstance(session_id, str)
        or not session_id.strip()
    ):
        return None
    turn_id = hook_input.get("turn_id")
    if hook_event == "PermissionRequest" and approvals_reviewer_for_turn(
        hook_input.get("transcript_path"), turn_id
    ) == "auto_review":
        state = "working"
    return AgentUpdate(
        session_id=session_id,
        state=state,
        event=EVENT_NAMES[hook_event],
        turn_id=turn_id if isinstance(turn_id, str) and turn_id.strip() else None,
    )


def build_clear_update(session_id: str) -> Any | None:
    if AgentUpdate is None:
        return None
    return AgentUpdate(
        session_id=session_id,
        state="clear",
        event="session_exit",
    )


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


def find_process_ancestors(start_pid: int | None = None) -> tuple[int | None, int | None]:
    pid = os.getppid() if start_pid is None else start_pid
    codex_pid = None
    zellij_pid = None
    for _ in range(PROCESS_ANCESTOR_LIMIT):
        info = process_info(pid)
        if info is None:
            break
        parent_pid, command = info
        command_name = Path(command).name.lower()
        if codex_pid is None and command_name in {"codex", "codex.exe"}:
            codex_pid = pid
        if command_name in {"zellij", "zellij.exe"}:
            zellij_pid = pid
            break
        if parent_pid <= 1 or parent_pid == pid:
            break
        pid = parent_pid
    return zellij_pid, codex_pid


def find_codex_ancestor(start_pid: int | None = None) -> int | None:
    return find_process_ancestors(start_pid)[1]


def watcher_directory(zellij_pid: int | None) -> Path:
    directory = server_directory(zellij_pid) if zellij_pid is not None else None
    if directory is None:
        directory = journal_root() / WATCHER_DIRECTORY / "unscoped"
    else:
        directory = directory / WATCHER_DIRECTORY
    return directory


def watcher_paths(zellij_pid: int | None, codex_pid: int) -> tuple[Path, Path]:
    directory = watcher_directory(zellij_pid)
    return directory / f"codex-{codex_pid}.lock", directory / f"codex-{codex_pid}.json"


def write_watcher_metadata(
    codex_pid: int,
    pane_id: str,
    session_id: str,
    zellij_pid: int | None,
) -> bool:
    lock_path, metadata_path = watcher_paths(zellij_pid, codex_pid)
    directory = lock_path.parent
    temporary_path: Path | None = None
    try:
        directory.mkdir(mode=0o700, parents=True, exist_ok=True)
        os.chmod(directory, 0o700)
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=directory,
            prefix=f".codex-{codex_pid}.",
            suffix=".tmp",
            delete=False,
        ) as temporary:
            temporary_path = Path(temporary.name)
            os.fchmod(temporary.fileno(), 0o600)
            json.dump(
                {
                    "pane_id": pane_id,
                    "session_id": session_id,
                    "zellij_pid": zellij_pid,
                },
                temporary,
                separators=(",", ":"),
            )
            temporary.write("\n")
        os.replace(temporary_path, metadata_path)
        return True
    except OSError:
        if temporary_path is not None:
            try:
                temporary_path.unlink(missing_ok=True)
            except OSError:
                pass
        return False


def read_watcher_metadata(
    codex_pid: int, zellij_pid: int | None
) -> tuple[str, str, int | None] | None:
    _, metadata_path = watcher_paths(zellij_pid, codex_pid)
    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    if not isinstance(metadata, dict):
        return None
    pane_id = metadata.get("pane_id")
    session_id = metadata.get("session_id")
    metadata_zellij_pid = metadata.get("zellij_pid")
    if (
        not isinstance(pane_id, str)
        or not pane_id.strip()
        or not isinstance(session_id, str)
        or not session_id.strip()
        or (metadata_zellij_pid is not None and parse_positive_pid(metadata_zellij_pid) is None)
    ):
        return None
    return pane_id, session_id, parse_positive_pid(metadata_zellij_pid)


def acquire_watcher_lock(
    codex_pid: int, zellij_pid: int | None
) -> tuple[BinaryIO | None, bool]:
    lock_path, _ = watcher_paths(zellij_pid, codex_pid)
    try:
        lock_path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        lock_file = lock_path.open("a+b")
        os.chmod(lock_path, 0o600)
        try:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            lock_file.close()
            return None, False
        return lock_file, True
    except OSError:
        return None, False


def wait_for_process_exit(pid: int) -> None:
    if not process_is_running(pid):
        return
    kqueue_factory = getattr(select, "kqueue", None)
    if kqueue_factory is not None:
        queue = None
        try:
            queue = kqueue_factory()
            event = select.kevent(
                pid,
                filter=select.KQ_FILTER_PROC,
                flags=select.KQ_EV_ADD | select.KQ_EV_ENABLE | select.KQ_EV_ONESHOT,
                fflags=select.KQ_NOTE_EXIT,
            )
            queue.control([event], 1, None)
            return
        except OSError as error:
            if error.errno in {errno.ESRCH, errno.ENOENT}:
                return
        finally:
            if queue is not None:
                queue.close()
    while process_is_running(pid):
        time.sleep(WATCHER_POLL_INTERVAL_SECONDS)


def watch_process(
    pid: int, pane_id: str, session_id: str, zellij_pid: int | None = None
) -> None:
    lock_file, should_watch = acquire_watcher_lock(pid, zellij_pid)
    if not should_watch:
        return
    try:
        wait_for_process_exit(pid)
        metadata = read_watcher_metadata(pid, zellij_pid)
        if metadata is not None:
            pane_id, session_id, zellij_pid = metadata
        update = build_clear_update(session_id)
        if update is not None:
            dispatch_update(
                update,
                pane_id,
                zellij_pid,
                discover_zellij=False,
            )
    finally:
        if lock_file is not None:
            lock_file.close()


def start_exit_watcher(
    pid: int, pane_id: str, session_id: str, zellij_pid: int | None = None
) -> None:
    write_watcher_metadata(pid, pane_id, session_id, zellij_pid)
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
    if not isinstance(hook_input, dict):
        return 0

    update = build_update(hook_input)
    if update is None:
        return 0
    zellij_pid, codex_pid = find_process_ancestors()
    payload = dispatch_update(
        update,
        pane_id,
        zellij_pid,
        discover_zellij=False,
        prune_dead=hook_input.get("hook_event_name") == "SessionStart",
    )
    if payload is None:
        return 0
    if hook_input.get("hook_event_name") == "SessionStart":
        if codex_pid is not None:
            start_exit_watcher(codex_pid, pane_id, payload["session_id"], zellij_pid)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
