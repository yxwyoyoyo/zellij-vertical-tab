## Why

The host lifecycle journal stores one `terminal_<pane_id>.json` record per terminal pane within each Zellij server PID directory. When a Zellij session is restarted, the server process gets a new PID and `prune_dead_server_directories` removes the old directory. However, when the same Zellij server persists across plugin restarts (hot reload, tab re-creation, session switch within the same server), the server PID stays the same while pane IDs change. Records from previous plugin attachments accumulate indefinitely — only `"clear"` tombstones and stale `"done"`/`"working"` records remain on disk with no path to removal.

A `SessionStart` event already triggers `prune_dead_server_directories`, making it the natural point to also clean up stale pane-level records within the live server directory.

## What Changes

- Add `prune_stale_pane_records(zellij_pid, keep_session_id)` to `status_store.py` that removes:
  - Records with `"clear"` state — explicitly expired via `SessionEnd`, always safe to delete.
  - Records older than a 6-hour grace period from a different session — crash zombies that were never properly cleared.
  - Records matching `keep_session_id` are always preserved (current or concurrent sessions).
- Call `prune_stale_pane_records` from `dispatch_update` when `prune_dead=True` (every `SessionStart`).
- Add Python tests for the new function covering clear-only, clear+stale, and noop-on-missing-directory cases.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Extend the durable lifecycle journal maintenance boundary to prune stale pane records within live server directories.

## Impact

- `hooks/common/status_store.py`: new `prune_stale_pane_records` function, `STALE_RECORD_GRACE_MS` constant, `time` import, helper path/lock functions.
- `hooks/common/agent_bridge.py`: import and call `prune_stale_pane_records` in `dispatch_update` when `prune_dead=True`.
- `hooks/common/test_status_store.py`: three new tests covering the prune function.
- Existing behavior for `load_snapshot`, `apply_payload`, and `prune_dead_server_directories` is unchanged.
