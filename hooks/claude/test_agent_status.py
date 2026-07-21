import importlib.util
import io
import json
import os
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("agent_status.py")
SPEC = importlib.util.spec_from_file_location("claude_agent_status", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
AGENT_STATUS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AGENT_STATUS)


class ClaudeAgentStatusHookTests(unittest.TestCase):
    def payload_for(self, event: str, prompt_id: str | None = "prompt-1"):
        hook_input = {"hook_event_name": event, "session_id": "session"}
        if prompt_id is not None:
            hook_input["prompt_id"] = prompt_id
        return AGENT_STATUS.build_payload(hook_input, "7")

    def test_lifecycle_mapping(self):
        expected = {
            "SessionStart": "idle",
            "UserPromptSubmit": "working",
            "PreToolUse": "working",
            "PermissionRequest": "waiting",
            "PostToolUse": "working",
            "PostToolUseFailure": "working",
            "PermissionDenied": "working",
            "Stop": "done",
            "SessionEnd": "clear",
        }
        self.assertEqual(
            {event: self.payload_for(event)["state"] for event in expected},
            expected,
        )

    def test_prompt_id_becomes_turn_identity(self):
        payload = self.payload_for("PostToolUse", "prompt-1")
        self.assertEqual(payload["turn_id"], "prompt-1")
        self.assertNotIn("turn_id", self.payload_for("PostToolUse", ""))

    def test_session_end_is_matching_session_clear(self):
        payload = self.payload_for("SessionEnd")
        self.assertEqual(payload["session_id"], "session")
        self.assertEqual(payload["event"], "session_exit")
        self.assertNotIn("turn_id", payload)

    def test_invalid_input_and_unknown_events_are_ignored(self):
        self.assertIsNone(AGENT_STATUS.build_payload({}, "7"))
        self.assertIsNone(
            AGENT_STATUS.build_payload(
                {"hook_event_name": "Unknown", "session_id": "session"}, "7"
            )
        )
        self.assertIsNone(
            AGENT_STATUS.build_payload(
                {"hook_event_name": "Stop", "session_id": ""}, "7"
            )
        )

    def test_process_hook_persists_before_publishing(self):
        calls = []
        with (
            patch.object(AGENT_STATUS, "find_zellij_ancestor", return_value=123),
            patch.object(
                AGENT_STATUS,
                "persist_payload",
                side_effect=lambda payload, pid: calls.append(("persist", payload, pid)),
            ),
            patch.object(
                AGENT_STATUS,
                "publish_payload",
                side_effect=lambda payload: calls.append(("publish", payload)),
            ),
        ):
            self.assertTrue(
                AGENT_STATUS.process_hook(
                    {
                        "hook_event_name": "Stop",
                        "session_id": "session",
                        "prompt_id": "prompt-1",
                    },
                    "7",
                )
            )
        self.assertEqual([call[0] for call in calls], ["persist", "publish"])
        self.assertEqual(calls[0][2], 123)
        self.assertEqual(calls[0][1], calls[1][1])

    def test_permission_and_final_stop_emit_bell(self):
        expected = {"terminalSequence": "\a"}
        self.assertEqual(
            AGENT_STATUS.notification_output({"hook_event_name": "PermissionRequest"}),
            expected,
        )
        self.assertEqual(
            AGENT_STATUS.notification_output({"hook_event_name": "Stop"}), expected
        )

    def test_continuing_stop_and_other_events_do_not_emit_bell(self):
        self.assertIsNone(
            AGENT_STATUS.notification_output(
                {"hook_event_name": "Stop", "stop_hook_active": True}
            )
        )
        self.assertIsNone(
            AGENT_STATUS.notification_output({"hook_event_name": "PostToolUse"})
        )
        self.assertIsNone(AGENT_STATUS.notification_output("invalid"))

    def test_main_returns_terminal_sequence_without_altering_status_publication(self):
        hook_input = {
            "hook_event_name": "PermissionRequest",
            "session_id": "session",
            "prompt_id": "prompt-1",
        }
        stdout = io.StringIO()
        with (
            patch.dict(os.environ, {"ZELLIJ_PANE_ID": "7"}, clear=True),
            patch.object(AGENT_STATUS.sys, "stdin", io.StringIO(json.dumps(hook_input))),
            patch.object(AGENT_STATUS.sys, "stdout", stdout),
            patch.object(AGENT_STATUS, "process_hook", return_value=True) as process,
        ):
            self.assertEqual(AGENT_STATUS.main(), 0)
        process.assert_called_once_with(hook_input, "7")
        self.assertEqual(json.loads(stdout.getvalue()), {"terminalSequence": "\a"})

    def test_main_outside_zellij_does_not_read_or_publish(self):
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(AGENT_STATUS.sys, "stdin", io.StringIO("not json")),
            patch.object(AGENT_STATUS, "publish_payload") as publish,
        ):
            self.assertEqual(AGENT_STATUS.main(), 0)
        publish.assert_not_called()

    def test_malformed_input_never_fails_hook(self):
        with (
            patch.dict(os.environ, {"ZELLIJ_PANE_ID": "7"}, clear=True),
            patch.object(AGENT_STATUS.sys, "stdin", io.StringIO("not json")),
            patch.object(AGENT_STATUS, "publish_payload") as publish,
        ):
            self.assertEqual(AGENT_STATUS.main(), 0)
        publish.assert_not_called()

    def test_settings_template_is_valid_and_covers_supported_events(self):
        settings = json.loads(Path(__file__).with_name("settings.json").read_text())
        self.assertEqual(set(settings["hooks"]), {
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionDenied",
            "Stop",
            "SessionEnd",
        })


if __name__ == "__main__":
    unittest.main()
