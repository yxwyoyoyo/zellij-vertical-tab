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
        update = AGENT_NOTIFY.build_done_update(
            json.dumps({"type": "agent-turn-complete", "thread-id": "thread"})
        )
        self.assertEqual(update.session_id, "thread")
        self.assertEqual(update.state, "done")
        self.assertEqual(update.event, "agent_turn_complete")

    def test_preserves_notification_turn_identity_when_available(self):
        update = AGENT_NOTIFY.build_done_update(
            json.dumps(
                {
                    "type": "agent-turn-complete",
                    "thread-id": "thread",
                    "turn-id": "turn-1",
                }
            )
        )
        self.assertEqual(update.turn_id, "turn-1")

    def test_ignores_other_or_invalid_notifications(self):
        self.assertIsNone(AGENT_NOTIFY.build_done_update("not-json"))
        self.assertIsNone(
            AGENT_NOTIFY.build_done_update(json.dumps({"type": "other"}))
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

    def test_main_persists_done_before_publication(self):
        raw = json.dumps({"type": "agent-turn-complete", "thread-id": "thread"})
        with (
            patch.object(AGENT_NOTIFY.sys, "argv", ["agent_notify.py", raw]),
            patch.dict(AGENT_NOTIFY.os.environ, {"ZELLIJ_PANE_ID": "7"}),
            patch.object(AGENT_NOTIFY, "dispatch_update") as dispatch,
        ):
            self.assertEqual(AGENT_NOTIFY.main(), 0)
        update = dispatch.call_args.args[0]
        self.assertEqual(update.session_id, "thread")
        self.assertEqual(update.state, "done")
        self.assertEqual(update.event, "agent_turn_complete")


if __name__ == "__main__":
    unittest.main()
