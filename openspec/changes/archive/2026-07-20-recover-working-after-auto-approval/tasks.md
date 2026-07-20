## 1. Lifecycle bridge

- [x] 1.1 Register and map `PostToolUse` to working
- [x] 1.2 Add normalized optional lifecycle event and turn identity to hook, notification, and exit records
- [x] 1.3 Apply terminal-done ordering in the host journal while preserving legacy version-1 records
- [x] 1.4 Add Python tests for automatic approval recovery, metadata validation, new turns, and delayed events
- [x] 1.5 Distinguish auto-review from manual permission using exact-turn transcript context with a bounded conservative fallback

## 2. Plugin ordering

- [x] 2.1 Parse, persist, and synchronize optional lifecycle event and turn identity
- [x] 2.2 Return waiting to working after PostToolUse and keep done terminal within a turn
- [x] 2.3 Preserve backward compatibility for legacy live and persistent version-1 records
- [x] 2.4 Add Rust tests for approval recovery, delayed hooks, missing metadata, and new-turn restart

## 3. Documentation

- [x] 3.1 Update README and OpenWiki lifecycle and troubleshooting guidance
- [x] 3.2 Regenerate OpenWiki and validate generated wording

## 4. Verification and deployment

- [x] 4.1 Run focused Python and Rust tests
- [x] 4.2 Run `mise run check`, strict OpenSpec validation, and `git diff --check`
- [x] 4.3 Install the updated global hooks and release plugin, then hot-reload the live session
- [x] 4.4 Reset status and verify waiting to working plus terminal-done ordering
- [x] 4.5 Install the corrected bridge and verify a real auto-reviewed request remains working

## Verification evidence

- `mise run check` passed 55 Rust tests, 22 Python tests, Clippy with warnings denied, the `wasm32-wasip1` build, strict validation of both baseline specs and this change, and `git diff --check`.
- `mise run deploy -- Hub` built and installed the release WASM and hot-reloaded it into the live session. The updated lifecycle bridge, notifier, journal, and global hook configuration were installed under `~/.codex/`.
- On `terminal_1` in `Hub`, a version-1 `permission_request` record rendered waiting, a same-turn `post_tool_use` record changed it to working, and stop rendered done. A newer delayed same-turn `post_tool_use` record did not replace done. A matching clear removed the test status before the temporary client detached.
- A real auto-reviewed unified-exec request on `terminal_2` disproved the original PostToolUse-only assumption: Codex displayed working while the journal remained on `permission_request` waiting, and no PostToolUse arrived when the command began. Its exact matching `turn_context` identified `approvals_reviewer` as `auto_review`, which now drives the corrected transition.
- The corrected bridge was installed at `~/.codex/hooks/agent_status.py`; its exact-turn lookup resolved the captured auto-reviewed request as `working`, and the follow-up user verification accepted the behavior for wrap-up.
