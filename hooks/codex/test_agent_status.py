import importlib.util
import json
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
    def payload_for(self, event: str, turn_id: str | None = None):
        hook_input = {"hook_event_name": event, "session_id": "session"}
        if turn_id is not None:
            hook_input["turn_id"] = turn_id
        return AGENT_STATUS.build_payload(
            hook_input, "7"
        )

    def test_lifecycle_mapping(self):
        self.assertEqual(self.payload_for("SessionStart")["state"], "idle")
        self.assertEqual(self.payload_for("UserPromptSubmit")["state"], "working")
        self.assertEqual(self.payload_for("PreToolUse")["state"], "working")
        self.assertEqual(self.payload_for("PostToolUse")["state"], "working")
        self.assertEqual(self.payload_for("PermissionRequest")["state"], "waiting")
        self.assertEqual(self.payload_for("Stop")["state"], "done")

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
            payload = AGENT_STATUS.build_payload(
                {
                    "hook_event_name": "PermissionRequest",
                    "session_id": "session",
                    "turn_id": "turn-1",
                    "transcript_path": transcript.name,
                },
                "7",
            )
        self.assertEqual(payload["state"], "working")
        self.assertEqual(payload["event"], "permission_request")

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
            manual = AGENT_STATUS.build_payload(
                {
                    "hook_event_name": "PermissionRequest",
                    "session_id": "session",
                    "turn_id": "turn-1",
                    "transcript_path": transcript.name,
                },
                "7",
            )
        unreadable = AGENT_STATUS.build_payload(
            {
                "hook_event_name": "PermissionRequest",
                "session_id": "session",
                "turn_id": "turn-1",
                "transcript_path": "/missing/transcript.jsonl",
            },
            "7",
        )
        self.assertEqual(manual["state"], "waiting")
        self.assertEqual(unreadable["state"], "waiting")

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
        payload = self.payload_for("PostToolUse", "turn-1")
        self.assertEqual(payload["event"], "post_tool_use")
        self.assertEqual(payload["turn_id"], "turn-1")
        self.assertNotIn("turn_id", self.payload_for("PostToolUse", ""))

    def test_invalid_input_is_ignored(self):
        self.assertIsNone(AGENT_STATUS.build_payload({}, "7"))
        self.assertIsNone(
            AGENT_STATUS.build_payload(
                {"hook_event_name": "Stop", "session_id": ""}, "7"
            )
        )

    def test_finds_nearest_codex_ancestor(self):
        with patch.object(
            AGENT_STATUS,
            "process_info",
            side_effect=[(20, "/bin/sh"), (10, "/opt/bin/codex")],
        ) as info:
            self.assertEqual(AGENT_STATUS.find_codex_ancestor(30), 20)
        self.assertEqual(info.call_args_list, [call(30), call(20)])

    def test_watcher_clears_matching_session_after_process_exits(self):
        with (
            patch.object(
                AGENT_STATUS, "process_is_running", side_effect=[True, False]
            ),
            patch.object(AGENT_STATUS.time, "sleep") as sleep,
            patch.object(AGENT_STATUS, "persist_payload") as persist,
            patch.object(AGENT_STATUS, "publish_payload") as publish,
        ):
            AGENT_STATUS.watch_process(42, "7", "session", 123)
        sleep.assert_called_once_with(0.5)
        payload = publish.call_args.args[0]
        self.assertEqual(payload["pane_id"], "7")
        self.assertEqual(payload["session_id"], "session")
        self.assertEqual(payload["state"], "clear")
        self.assertEqual(payload["event"], "session_exit")
        persist.assert_called_once_with(payload, 123)


if __name__ == "__main__":
    unittest.main()
