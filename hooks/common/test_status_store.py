"""Contract tests for the common durable agent-status journal."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import call, patch

import status_store


def payload(
    timestamp: int,
    state: str = "working",
    session_id: str = "session",
    pane_id: str = "terminal_7",
    event: str | None = None,
    turn_id: str | None = None,
):
    result = {
        "version": 1,
        "pane_id": pane_id,
        "session_id": session_id,
        "state": state,
        "updated_at_ms": timestamp,
    }
    if event is not None:
        result["event"] = event
    if turn_id is not None:
        result["turn_id"] = turn_id
    return result


class StatusStoreTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.environment = patch.dict(
            os.environ,
            {status_store.STATE_DIR_ENV: self.temporary.name},
        )
        self.environment.start()

    def tearDown(self):
        self.environment.stop()
        self.temporary.cleanup()

    def test_validates_and_normalizes_payload(self):
        self.assertEqual(
            status_store.validate_payload(payload(10, pane_id="7"))["pane_id"],
            "terminal_7",
        )
        for invalid in [
            {},
            payload(0),
            payload(10, state="unknown"),
            payload(10, session_id=""),
            payload(10, pane_id="plugin_7"),
            payload(10, event="unknown"),
            payload(10, turn_id=""),
        ]:
            self.assertIsNone(status_store.validate_payload(invalid))

    def test_preserves_supported_event_and_turn_identity(self):
        record = payload(10, event="post_tool_use", turn_id="turn-1")
        self.assertEqual(status_store.validate_payload(record), record)

    def test_retains_newest_record_and_matching_session_clear(self):
        self.assertTrue(status_store.apply_payload(payload(10), 123))
        self.assertFalse(status_store.apply_payload(payload(9, state="done"), 123))
        self.assertFalse(
            status_store.apply_payload(
                payload(11, state="clear", session_id="old"), 123
            )
        )
        self.assertTrue(status_store.apply_payload(payload(12, state="clear"), 123))

        snapshot = status_store.load_snapshot(123)
        self.assertEqual(snapshot["records"], [payload(12, state="clear")])
        record_path = (
            Path(self.temporary.name) / "sessions" / "123" / "terminal_7.json"
        )
        self.assertEqual(record_path.stat().st_mode & 0o777, 0o600)

    def test_done_is_terminal_within_turn_and_post_tool_recovers_waiting(self):
        waiting = payload(
            10, state="waiting", event="permission_request", turn_id="turn-1"
        )
        resumed = payload(11, event="post_tool_use", turn_id="turn-1")
        done = payload(12, state="done", event="stop", turn_id="turn-1")
        delayed = payload(13, event="post_tool_use", turn_id="turn-1")

        self.assertTrue(status_store.apply_payload(waiting, 123))
        self.assertTrue(status_store.apply_payload(resumed, 123))
        self.assertTrue(status_store.apply_payload(done, 123))
        self.assertFalse(status_store.apply_payload(delayed, 123))
        self.assertEqual(status_store.load_snapshot(123)["records"], [done])

    def test_new_turn_reopens_done_with_and_without_prior_turn_identity(self):
        for zellij_pid, done in [
            (123, payload(10, state="done", event="stop", turn_id="turn-1")),
            (124, payload(10, state="done")),
        ]:
            self.assertTrue(status_store.apply_payload(done, zellij_pid))
            self.assertFalse(
                status_store.apply_payload(
                    payload(11, event="post_tool_use", turn_id="turn-1"), zellij_pid
                )
            )
            new_prompt = payload(
                12, event="user_prompt_submit", turn_id="turn-2"
            )
            self.assertTrue(status_store.apply_payload(new_prompt, zellij_pid))
            self.assertEqual(
                status_store.load_snapshot(zellij_pid)["records"], [new_prompt]
            )

    def test_snapshot_skips_malformed_mismatched_and_oversized_files(self):
        directory = Path(self.temporary.name) / "sessions" / "123"
        directory.mkdir(parents=True)
        (directory / "terminal_1.json").write_text("not json")
        (directory / "terminal_2.json").write_text(json.dumps(payload(10)))
        (directory / "terminal_3.json").write_bytes(
            b"x" * (status_store.MAX_RECORD_BYTES + 1)
        )
        (directory / "terminal_7.json").write_text(json.dumps(payload(11)))

        snapshot = status_store.load_snapshot(123)
        self.assertEqual(snapshot["records"], [payload(11)])
        self.assertEqual(snapshot["acknowledgements"], [])

    def test_concurrent_processes_retain_highest_timestamp(self):
        module_directory = str(Path(status_store.__file__).parent)
        child_environment = os.environ.copy()
        child_environment["PYTHONPATH"] = module_directory
        code = (
            "import sys; from status_store import apply_payload; "
            "timestamp=int(sys.argv[1]); "
            "apply_payload({'version':1,'pane_id':'terminal_7','session_id':'session',"
            "'state':'working','updated_at_ms':timestamp},321)"
        )
        processes = [
            subprocess.Popen(
                [sys.executable, "-c", code, str(timestamp)],
                env=child_environment,
            )
            for timestamp in range(20, 30)
        ]
        self.assertTrue(all(process.wait(timeout=5) == 0 for process in processes))
        self.assertEqual(
            status_store.load_snapshot(321)["records"],
            [payload(29)],
        )

    def test_finds_nearest_zellij_ancestor(self):
        with patch.object(
            status_store,
            "process_info",
            side_effect=[(20, "/opt/bin/codex"), (10, "/usr/bin/zellij")],
        ) as info:
            self.assertEqual(status_store.find_zellij_ancestor(30), 20)
        self.assertEqual(info.call_args_list, [call(30), call(20)])

    def test_prunes_only_demonstrably_dead_server_directories(self):
        sessions = Path(self.temporary.name) / "sessions"
        for name in ("100", "101", "102", "not-a-pid"):
            directory = sessions / name
            directory.mkdir(parents=True)
            (directory / "marker").write_text(name)
        with patch.object(
            status_store,
            "process_is_running",
            side_effect=lambda pid: pid == 102,
        ):
            self.assertEqual(status_store.prune_dead_server_directories(100), 1)
        self.assertTrue((sessions / "100").is_dir())
        self.assertFalse((sessions / "101").exists())
        self.assertTrue((sessions / "102").is_dir())
        self.assertTrue((sessions / "not-a-pid").is_dir())


    def test_prunes_clear_and_stale_pane_records(self):
        import time

        now_ms = time.time_ns() // 1_000_000
        old_ms = now_ms - (status_store.STALE_RECORD_GRACE_MS + 1000)
        recent_ms = now_ms - 1000

        sessions = Path(self.temporary.name) / "sessions" / "200"
        sessions.mkdir(parents=True)

        # Explicitly expired — should always be removed.
        clear_record = json.dumps(
            payload(old_ms, state="clear", session_id="dead", event="session_exit")
        )
        # Old record from different session — crash zombie.
        old_record = json.dumps(
            payload(old_ms, state="done", session_id="dead")
        )
        # Recent record from different session — keep (might still be alive).
        recent_other = json.dumps(
            payload(recent_ms, state="done", session_id="other")
        )
        # Recent record from current session — keep.
        current_rec = json.dumps(
            payload(recent_ms, state="working", session_id="current")
        )

        (sessions / "terminal_0.json").write_text(clear_record)
        (sessions / "terminal_1.json").write_text(old_record)
        (sessions / "terminal_2.json").write_text(recent_other)
        (sessions / "terminal_3.json").write_text(current_rec)

        removed = status_store.prune_stale_pane_records(200, keep_session_id="current")
        self.assertEqual(removed, 2)

        remaining = sorted(
            p.name for p in sessions.glob("terminal_*.json")
        )
        self.assertEqual(remaining, ["terminal_2.json", "terminal_3.json"])

    def test_prune_without_session_id_only_removes_clear(self):
        import time

        now_ms = time.time_ns() // 1_000_000
        old_ms = now_ms - (status_store.STALE_RECORD_GRACE_MS + 1000)

        sessions = Path(self.temporary.name) / "sessions" / "300"
        sessions.mkdir(parents=True)

        clear_record = json.dumps(
            payload(old_ms, state="clear", session_id="dead", event="session_exit")
        )
        # Old "done" record — should NOT be removed without keep_session_id
        old_done = json.dumps(
            payload(old_ms, state="done", session_id="dead")
        )

        (sessions / "terminal_0.json").write_text(clear_record)
        (sessions / "terminal_1.json").write_text(old_done)

        removed = status_store.prune_stale_pane_records(300)
        self.assertEqual(removed, 1)
        remaining = sorted(
            p.name for p in sessions.glob("terminal_*.json")
        )
        self.assertEqual(remaining, ["terminal_1.json"])

    def test_prune_stale_pane_records_is_noop_for_missing_directory(self):
        self.assertEqual(status_store.prune_stale_pane_records(99999), 0)


if __name__ == "__main__":
    unittest.main()
