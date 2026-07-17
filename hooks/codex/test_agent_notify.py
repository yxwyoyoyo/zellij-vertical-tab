import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("agent_notify.py")
SPEC = importlib.util.spec_from_file_location("agent_notify", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
AGENT_NOTIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AGENT_NOTIFY)


class AgentNotifyTests(unittest.TestCase):
    def test_builds_done_from_turn_complete(self):
        payload = AGENT_NOTIFY.build_done_payload(
            json.dumps({"type": "agent-turn-complete", "thread-id": "thread"}), "7"
        )
        self.assertEqual(payload["pane_id"], "7")
        self.assertEqual(payload["session_id"], "thread")
        self.assertEqual(payload["state"], "done")

    def test_ignores_other_or_invalid_notifications(self):
        self.assertIsNone(AGENT_NOTIFY.build_done_payload("not-json", "7"))
        self.assertIsNone(
            AGENT_NOTIFY.build_done_payload(json.dumps({"type": "other"}), "7")
        )

    def test_splits_forward_command_without_modifying_payload(self):
        self.assertEqual(
            AGENT_NOTIFY.split_arguments(
                ["--forward", "/notifier", "turn-ended", "--", "payload"]
            ),
            (["/notifier", "turn-ended"], "payload"),
        )
        self.assertIsNone(
            AGENT_NOTIFY.split_arguments(["--forward", "/notifier", "payload"])
        )

    def test_forwards_payload_as_final_argument(self):
        with patch.object(AGENT_NOTIFY.subprocess, "run") as run:
            AGENT_NOTIFY.forward_notification(["/notifier", "turn-ended"], "payload")
        self.assertEqual(
            run.call_args.args[0], ["/notifier", "turn-ended", "payload"]
        )


if __name__ == "__main__":
    unittest.main()
