import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import agent_bridge


class AgentBridgeTests(unittest.TestCase):
    def update(self, **overrides):
        values = {
            "session_id": "session",
            "state": "working",
            "event": "user_prompt_submit",
            "turn_id": "turn-1",
        }
        values.update(overrides)
        return agent_bridge.AgentUpdate(**values)

    def test_builds_valid_versioned_payload_without_adapter_owned_timestamp(self):
        payload = agent_bridge.build_payload(self.update(), "7", updated_at_ms=42)
        self.assertEqual(
            payload,
            {
                "version": 1,
                "pane_id": "7",
                "session_id": "session",
                "state": "working",
                "updated_at_ms": 42,
                "event": "user_prompt_submit",
                "turn_id": "turn-1",
            },
        )

    def test_rejects_invalid_normalized_updates(self):
        for update in (
            self.update(session_id=""),
            self.update(state="unknown"),
            self.update(event="unknown"),
            self.update(turn_id=""),
        ):
            self.assertIsNone(agent_bridge.build_payload(update, "7", 42))

    def test_dispatch_persists_before_publishing(self):
        calls = []
        with (
            patch.object(agent_bridge, "find_zellij_ancestor", return_value=123),
            patch.object(
                agent_bridge,
                "persist_payload",
                side_effect=lambda payload, pid: calls.append(("persist", payload, pid)),
            ),
            patch.object(
                agent_bridge,
                "publish_payload",
                side_effect=lambda payload: calls.append(("publish", payload)),
            ),
        ):
            payload = agent_bridge.dispatch_update(self.update(), "7")
        self.assertIsNotNone(payload)
        self.assertEqual([call[0] for call in calls], ["persist", "publish"])
        self.assertEqual(calls[0][2], 123)
        self.assertEqual(calls[0][1], calls[1][1])

    def test_session_start_can_prune_before_persistence(self):
        calls = []
        with (
            patch.object(agent_bridge, "find_zellij_ancestor", return_value=123),
            patch.object(
                agent_bridge,
                "prune_dead_server_directories",
                side_effect=lambda pid: calls.append(("prune", pid)),
            ),
            patch.object(
                agent_bridge,
                "persist_payload",
                side_effect=lambda payload, pid: calls.append(("persist", pid)),
            ),
            patch.object(agent_bridge, "publish_payload"),
        ):
            agent_bridge.dispatch_update(self.update(), "7", prune_dead=True)
        self.assertEqual(calls, [("prune", 123), ("persist", 123)])

    def test_outside_zellij_does_not_build_store_or_publish(self):
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(agent_bridge, "build_payload") as build,
            patch.object(agent_bridge, "persist_payload") as persist,
            patch.object(agent_bridge, "publish_payload") as publish,
        ):
            self.assertIsNone(agent_bridge.dispatch_update(self.update()))
        build.assert_not_called()
        persist.assert_not_called()
        publish.assert_not_called()

    def test_snapshot_delegates_only_for_positive_pid(self):
        with patch.object(agent_bridge, "snapshot_json", return_value="snapshot") as snapshot:
            self.assertEqual(agent_bridge.snapshot_for_argument("123"), "snapshot")
            self.assertIsNone(agent_bridge.snapshot_for_argument("invalid"))
        snapshot.assert_called_once_with(123)

    def test_publication_failure_is_best_effort(self):
        with patch.object(
            agent_bridge.subprocess, "run", side_effect=OSError("missing")
        ):
            agent_bridge.publish_payload({"version": 1})

    def test_common_modules_work_when_colocated_with_each_adapter(self):
        hooks_directory = Path(__file__).resolve().parent.parent
        for adapter in (
            hooks_directory / "codex" / "agent_status.py",
            hooks_directory / "claude" / "agent_status.py",
        ):
            with self.subTest(adapter=adapter.parent.name):
                with tempfile.TemporaryDirectory() as directory:
                    install_directory = Path(directory)
                    for source in (
                        Path(agent_bridge.__file__),
                        Path(agent_bridge.__file__).with_name("status_store.py"),
                        adapter,
                    ):
                        shutil.copy2(source, install_directory / source.name)
                    environment = os.environ.copy()
                    environment.pop("PYTHONPATH", None)
                    result = subprocess.run(
                        [
                            sys.executable,
                            str(install_directory / "agent_status.py"),
                            "--snapshot",
                            "invalid",
                        ],
                        cwd=install_directory,
                        env=environment,
                        check=False,
                        capture_output=True,
                        text=True,
                        timeout=5,
                    )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()
