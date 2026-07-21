import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import call, patch


MODULE_PATH = Path(__file__).with_name("agent_status.py")
SPEC = importlib.util.spec_from_file_location("agent_status", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
AGENT_STATUS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AGENT_STATUS)


class AgentStatusHookTests(unittest.TestCase):
    def update_for(self, event: str, turn_id: str | None = None):
        hook_input = {"hook_event_name": event, "session_id": "session"}
        if turn_id is not None:
            hook_input["turn_id"] = turn_id
        return AGENT_STATUS.build_update(hook_input)

    def test_lifecycle_mapping(self):
        self.assertEqual(self.update_for("SessionStart").state, "idle")
        self.assertEqual(self.update_for("UserPromptSubmit").state, "working")
        self.assertEqual(self.update_for("PreToolUse").state, "working")
        self.assertEqual(self.update_for("PostToolUse").state, "working")
        self.assertEqual(self.update_for("PermissionRequest").state, "waiting")
        self.assertEqual(self.update_for("Stop").state, "done")

    def test_auto_review_permission_remains_working(self):
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8") as transcript:
            transcript.write(
                json.dumps(
                    {
                        "type": "turn_context",
                        "payload": {
                            "turn_id": "turn-1",
                            "approvals_reviewer": "auto_review",
                        },
                    }
                )
                + "\n"
            )
            transcript.flush()
            update = AGENT_STATUS.build_update(
                {
                    "hook_event_name": "PermissionRequest",
                    "session_id": "session",
                    "turn_id": "turn-1",
                    "transcript_path": transcript.name,
                },
            )
        self.assertEqual(update.state, "working")
        self.assertEqual(update.event, "permission_request")

    def test_manual_or_unreadable_permission_waits(self):
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8") as transcript:
            transcript.write(
                json.dumps(
                    {
                        "type": "turn_context",
                        "payload": {
                            "turn_id": "turn-1",
                            "approvals_reviewer": "user",
                        },
                    }
                )
                + "\n"
            )
            transcript.flush()
            manual = AGENT_STATUS.build_update(
                {
                    "hook_event_name": "PermissionRequest",
                    "session_id": "session",
                    "turn_id": "turn-1",
                    "transcript_path": transcript.name,
                },
            )
        unreadable = AGENT_STATUS.build_update(
            {
                "hook_event_name": "PermissionRequest",
                "session_id": "session",
                "turn_id": "turn-1",
                "transcript_path": "/missing/transcript.jsonl",
            },
        )
        self.assertEqual(manual.state, "waiting")
        self.assertEqual(unreadable.state, "waiting")

    def test_reviewer_lookup_uses_matching_turn(self):
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8") as transcript:
            for turn_id, reviewer in (
                ("turn-1", "auto_review"),
                ("turn-2", "user"),
            ):
                transcript.write(
                    json.dumps(
                        {
                            "type": "turn_context",
                            "payload": {
                                "turn_id": turn_id,
                                "approvals_reviewer": reviewer,
                            },
                        }
                    )
                    + "\n"
                )
            transcript.flush()
            self.assertEqual(
                AGENT_STATUS.approvals_reviewer_for_turn(transcript.name, "turn-1"),
                "auto_review",
            )

    def test_includes_normalized_event_and_non_empty_turn_identity(self):
        update = self.update_for("PostToolUse", "turn-1")
        self.assertEqual(update.event, "post_tool_use")
        self.assertEqual(update.turn_id, "turn-1")
        self.assertIsNone(self.update_for("PostToolUse", "").turn_id)

    def test_invalid_input_is_ignored(self):
        self.assertIsNone(AGENT_STATUS.build_update({}))
        self.assertIsNone(
            AGENT_STATUS.build_update({"hook_event_name": "Stop", "session_id": ""})
        )

    def test_finds_codex_and_zellij_ancestors_in_one_traversal(self):
        with patch.object(
            AGENT_STATUS,
            "process_info",
            side_effect=[
                (20, "/bin/sh"),
                (10, "/opt/bin/codex"),
                (1, "/usr/bin/zellij"),
            ],
        ) as info:
            self.assertEqual(AGENT_STATUS.find_process_ancestors(30), (10, 20))
        self.assertEqual(info.call_args_list, [call(30), call(20), call(10)])

    def test_watcher_clears_matching_session_after_process_exits(self):
        lock_file = unittest.mock.MagicMock()
        with (
            patch.object(
                AGENT_STATUS, "acquire_watcher_lock", return_value=(lock_file, True)
            ),
            patch.object(AGENT_STATUS, "wait_for_process_exit") as wait,
            patch.object(AGENT_STATUS, "read_watcher_metadata", return_value=None),
            patch.object(AGENT_STATUS, "dispatch_update") as dispatch,
        ):
            AGENT_STATUS.watch_process(42, "7", "session", 123)
        wait.assert_called_once_with(42)
        lock_file.close.assert_called_once_with()
        update = dispatch.call_args.args[0]
        self.assertEqual(update.session_id, "session")
        self.assertEqual(update.state, "clear")
        self.assertEqual(update.event, "session_exit")
        self.assertEqual(dispatch.call_args.args[1:3], ("7", 123))
        self.assertFalse(dispatch.call_args.kwargs["discover_zellij"])

    def test_duplicate_watcher_exits_without_waiting_or_clearing(self):
        with (
            patch.object(
                AGENT_STATUS, "acquire_watcher_lock", return_value=(None, False)
            ),
            patch.object(AGENT_STATUS, "wait_for_process_exit") as wait,
            patch.object(AGENT_STATUS, "dispatch_update") as dispatch,
        ):
            AGENT_STATUS.watch_process(42, "7", "session", 123)
        wait.assert_not_called()
        dispatch.assert_not_called()

    def test_repeated_start_refreshes_watcher_session_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(
                os.environ,
                {"ZELLIJ_VERTICAL_TAB_STATE_DIR": directory},
            ):
                self.assertTrue(
                    AGENT_STATUS.write_watcher_metadata(42, "7", "old-session", 123)
                )
                self.assertTrue(
                    AGENT_STATUS.write_watcher_metadata(42, "8", "new-session", 123)
                )
                self.assertEqual(
                    AGENT_STATUS.read_watcher_metadata(42, 123),
                    ("8", "new-session", 123),
                )

    def test_watcher_lock_allows_only_one_owner_per_process(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(
                os.environ,
                {"ZELLIJ_VERTICAL_TAB_STATE_DIR": directory},
            ):
                first_lock, first_should_watch = AGENT_STATUS.acquire_watcher_lock(
                    42, 123
                )
                second_lock, second_should_watch = AGENT_STATUS.acquire_watcher_lock(
                    42, 123
                )
                self.assertTrue(first_should_watch)
                self.assertIsNotNone(first_lock)
                self.assertFalse(second_should_watch)
                self.assertIsNone(second_lock)
                first_lock.close()
                replacement_lock, replacement_should_watch = (
                    AGENT_STATUS.acquire_watcher_lock(42, 123)
                )
                self.assertTrue(replacement_should_watch)
                replacement_lock.close()

    def test_process_exit_uses_kqueue_when_available(self):
        queue = unittest.mock.MagicMock()
        with (
            patch.object(AGENT_STATUS, "process_is_running", return_value=True),
            patch.object(AGENT_STATUS.select, "kqueue", return_value=queue),
            patch.object(AGENT_STATUS.select, "kevent", return_value="event") as kevent,
            patch.object(AGENT_STATUS.time, "sleep") as sleep,
        ):
            AGENT_STATUS.wait_for_process_exit(42)
        kevent.assert_called_once()
        queue.control.assert_called_once_with(["event"], 1, None)
        queue.close.assert_called_once_with()
        sleep.assert_not_called()

    def test_process_exit_falls_back_to_bounded_polling(self):
        with (
            patch.object(AGENT_STATUS.select, "kqueue", None),
            patch.object(
                AGENT_STATUS, "process_is_running", side_effect=[True, True, False]
            ),
            patch.object(AGENT_STATUS.time, "sleep") as sleep,
        ):
            AGENT_STATUS.wait_for_process_exit(42)
        sleep.assert_called_once_with(AGENT_STATUS.WATCHER_POLL_INTERVAL_SECONDS)


if __name__ == "__main__":
    unittest.main()
