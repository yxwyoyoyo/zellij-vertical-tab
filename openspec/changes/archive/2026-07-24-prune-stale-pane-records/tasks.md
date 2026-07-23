## 1. Pane-record pruning implementation

- [x] 1.1 Add `STALE_RECORD_GRACE_MS` constant and `time` import to `status_store.py`
- [x] 1.2 Add `_file_for_pane` and `_lock_for_pane` helpers to reduce path-construction duplication
- [x] 1.3 Implement `prune_stale_pane_records(zellij_pid, keep_session_id)` with lock-guarded removal of `"clear"` records and age-based zombie cleanup
- [x] 1.4 Call `prune_stale_pane_records` from `dispatch_update` in `agent_bridge.py` when `prune_dead=True` (SessionStart)
- [x] 1.5 Add `prune_stale_pane_records` to the import from `status_store` in `agent_bridge.py`

## 2. Testing

- [x] 2.1 Test that both `"clear"` records and old records from a different session are pruned when `keep_session_id` is provided
- [x] 2.2 Test that only `"clear"` records are pruned when `keep_session_id` is `None`
- [x] 2.3 Test that calling `prune_stale_pane_records` on a missing server directory is a noop
- [x] 2.4 Run full `mise run test` — 59 Rust + 50 Python tests pass

## 3. Documentation

- [x] 3.1 Create OpenSpec change artifacts (proposal, design, tasks, spec delta)
- [x] 3.2 Run OpenSpec validation — `openspec validate --all --strict` passed
- [ ] 3.3 Deploy to verify stale records are cleaned on next SessionStart
