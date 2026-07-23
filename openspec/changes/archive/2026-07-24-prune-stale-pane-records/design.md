## Context

`prune_dead_server_directories` handles the case where an entire Zellij server process dies — it removes the entire `sessions/<pid>` directory. But the journal also accumulates stale pane records *within* a live server directory. Each re-creation of the plugin pane (or the terminal pane running the agent) within the same Zellij server session gets a new pane ID, creating a new `terminal_<N>.json` file while the old one remains.

The existing `SessionStart` event already triggers `prune_dead=True` in `dispatch_update`, so adding pane-level pruning there reuses an established maintenance point without adding new lifecycle hooks or periodic timers.

## Goals / Non-Goals

**Goals:**

- Remove `"clear"` state records (explicit `SessionEnd` tombsones) on every `SessionStart`.
- Remove crash-zombie records (old records from a different session) after a 6-hour grace period, only when anchored by a known-live `keep_session_id`.
- Keep the current session's record and any concurrently active sessions' records untouched.
- Operate within the existing advisory-locking discipline so concurrent writers are safe.
- Add focused Python tests.

**Non-Goals:**

- Do not add periodic background cleanup or timers — pruning is event-driven on `SessionStart`.
- Do not change `load_snapshot`, `apply_payload`, or `prune_dead_server_directories` behavior.
- Do not add pane-aliveness detection via process-tree walking — the session-id-based guard is simpler and sufficient.
- Do not prune Rust-side `agent-cache-<plugin_id>.json` files in this change (those are functionally harmless and self-limiting because newer snapshots overwrite older data on restore).

## Decisions

### Anchor age-based pruning on `keep_session_id`

Without a session anchor, age-based pruning could remove records from still-active concurrent Claude Code sessions. When `keep_session_id` is `None`, only `"clear"` records are removed. When a session ID is provided (from `SessionStart`), records matching that session are preserved, and old records from *different* sessions beyond the 6-hour grace period are treated as crash zombies.

This is preferred to process-tree walking because it is cross-platform, requires no subprocess calls, and the session-ID guard provides a logically sound "I know this session is alive, so records from other sessions that are very old must be dead."

### 6-hour grace period

6 hours is long enough that a temporarily-suspended session or long-running detached work won't be incorrectly pruned, but short enough that accumulation across days of development is cleaned up. The grace period only applies to records with a *different* session ID from the currently-starting session — records from the current session are never age-pruned.

### Prune on `SessionStart` only

`SessionStart` already triggers `prune_dead_server_directories`. Adding pane-level pruning at the same point avoids introducing a new trigger path. `SessionStart` fires exactly once per Claude Code session, keeping overhead minimal.

## Risks / Trade-offs

- [A concurrent Claude Code session has been idle for 6+ hours] → Its record could be pruned on another session's `SessionStart`. Mitigation: a session idle for 6+ hours with no lifecycle events is effectively dead; if it resumes, it will write a fresh record on its next event.
- [Clock skew between writers] → Timestamp ordering already handles this at the `apply_payload` level; stale-record age checks use the pruning process's own monotonic clock and only affect cleanup, not correctness.
- [Pane ID is reused within the grace period] → The old record from a different session survives until its session-specific `clear` fires or the grace period expires, then is cleaned up. No functional impact — stale records are inert.

## Migration Plan

No migration required. The new function is additive. On the next `SessionStart` after deployment, stale `"clear"` records are removed immediately; zombie records age out after 6 hours. Existing journal records and all lifecycle paths are unchanged.

## Open Questions

None.
