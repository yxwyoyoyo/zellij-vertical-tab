import importlib.util
import unittest
from pathlib import Path
from unittest.mock import call, patch


MODULE_PATH = Path(__file__).with_name("agent_status.py")
SPEC = importlib.util.spec_from_file_location("agent_status", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
AGENT_STATUS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AGENT_STATUS)


class AgentStatusHookTests(unittest.TestCase):
    def payload_for(self, event: str):
        return AGENT_STATUS.build_payload(
            {"hook_event_name": event, "session_id": "session"}, "7"
        )

    def test_lifecycle_mapping(self):
        self.assertEqual(self.payload_for("SessionStart")["state"], "idle")
        self.assertEqual(self.payload_for("UserPromptSubmit")["state"], "working")
        self.assertEqual(self.payload_for("PreToolUse")["state"], "working")
        self.assertIsNone(self.payload_for("PostToolUse"))
        self.assertEqual(self.payload_for("PermissionRequest")["state"], "waiting")
        self.assertEqual(self.payload_for("Stop")["state"], "done")

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
            patch.object(AGENT_STATUS, "publish_payload") as publish,
        ):
            AGENT_STATUS.watch_process(42, "7", "session")
        sleep.assert_called_once_with(0.5)
        payload = publish.call_args.args[0]
        self.assertEqual(payload["pane_id"], "7")
        self.assertEqual(payload["session_id"], "session")
        self.assertEqual(payload["state"], "clear")


if __name__ == "__main__":
    unittest.main()
