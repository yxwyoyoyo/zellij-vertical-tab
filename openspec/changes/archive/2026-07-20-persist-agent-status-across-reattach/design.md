## Context

The plugin currently stores `AgentRecord`, exact focus acknowledgements, and peer convergence state only in WASM memory. Zellij 0.44.3 retains plugin pane IDs across detach and reattach but recreates the plugin runtime, so every per-tab instance restarts empty. A real one-pane reproduction rendered `done` before detach and no status after reattach. A second reproduction sent `working` while detached and confirmed that an undirected CLI pipe was not replayed when the client returned.

Zellij provides persistent per-plugin `/cache` storage and a permission-gated `run_command` API. Codex hooks execute on the host and already know the pane, Codex session, timestamps, and process ancestry. Recovery must combine these two boundaries: plugin cache preserves presentation details such as focus acknowledgement, while a host journal records lifecycle events that occur with no running plugin.

## Goals / Non-Goals

**Goals:**

- Preserve current lifecycle and acknowledgement presentation across detach, session switch, reattach, and hot reload.
- Capture lifecycle changes, especially `clear`, while all plugin runtimes are unloaded.
- Reuse the existing version-1 payload validation, timestamp ordering, session-reuse protection, clear tombstones, and peer snapshots.
- Keep publication and recovery best-effort so Codex and the sidebar remain usable if storage, permission, or helper execution fails.
- Prevent state from one live Zellij server from appearing in another server that reuses pane IDs.

**Non-Goals:**

- Do not reconstruct lifecycle events that occurred before this version first journals a status.
- Do not persist native Zellij bell state, focus baselines, scrolling, sidebar geometry, or pane contents.
- Do not guarantee recovery after the host cache is manually deleted or after a full Zellij session resurrection with a different server process.
- Do not replace the Python Codex bridges with Rust in this change.

## Decisions

### Journal host lifecycle records before publishing

Add a dependency-free `status_store.py` shared by both bridges. It finds the Zellij server in the Codex process ancestry and stores one validated version-1 payload per terminal pane beneath a positive server-PID namespace. Each pane file is protected by an advisory lock, merged with the same timestamp and session-safe clear rules as the plugin, and replaced atomically with mode `0600`. `clear` remains a tombstone instead of deleting the file.

One file per pane avoids unrelated Codex sessions contending on a session-wide read-modify-write document. The server PID separates simultaneously running sessions and session-name reuse. Missing ancestry disables persistence for that event but does not change normal pipe publication.

This is preferred to plugin-only `/cache`: a detached exit watcher cannot reach an unloaded undirected pipe, so plugin-only storage would restore stale `done`. It is preferred to a single session JSON file because concurrent pane writers could lose updates even with atomic replacement.

### Use plugin cache for immediate full-snapshot recovery

Each sidebar instance writes the existing serialized snapshot to its own `/cache/agent-status-<zellij-pid>-<plugin-id>.json` file after lifecycle, acknowledgement, snapshot, or cleanup changes. Writes use a temporary sibling and rename. On load, an instance scans all cache files for its Zellij server PID and timestamp-merges them before the first render.

Per-instance files avoid concurrent WASM writers targeting one path. Scanning all matching files tolerates a changed plugin ID and converges acknowledgement state produced by any tab. Pane updates still remove records for missing terminals. Focus observations are not persisted, so startup establishes a fresh baseline and cannot acknowledge a result by itself.

### Reconcile with the host journal through a fixed helper command

Extend `agent_status.py` with `--snapshot <zellij-pid>`, which prints the same version-1 `AgentStatusSnapshot` shape with empty acknowledgements. The plugin requests `RunCommands`, subscribes to permission and result events, and invokes the globally installed helper through a fixed `/bin/sh` command that resolves `${CODEX_HOME:-$HOME/.codex}/hooks/agent_status.py`. The server PID is passed as a positional argument rather than interpolated into shell source.

Only a successful result with the expected context marker, bounded UTF-8 stdout, and a fully valid snapshot is merged. Existing timestamp ordering makes recovery race-safe: a newer live pipe beats an older snapshot, while a detached newer `clear` beats cached `done`. Every sidebar may request recovery; read-only duplication is harmless and subsequent peer synchronization converges the result.

This adds one permission rather than granting full host-disk access to WASM. A missing helper or denied permission leaves the immediately restored plugin cache in place and preserves current best-effort behavior.

### Keep recovery data bounded and non-authoritative

The host snapshot returns only strictly validated pane files for the requested positive server PID and enforces a maximum file/result size. Plugin cache filenames accept digits only. Corrupt entries are skipped individually. Cache data never bypasses lifecycle validation or pane cleanup.

Old server-PID directories may remain as small cache artifacts. They are isolated from current sessions; bounded age-based cleanup can be added to the host writer without affecting correctness.

## Risks / Trade-offs

- [The helper is not installed or `RunCommands` is denied] → Restore plugin cache only, ignore the helper failure, and document that detached updates cannot then be reconciled.
- [The hook cannot identify a Zellij server ancestor] → Continue publishing normally and skip only the durable journal write.
- [A live event races snapshot recovery] → Merge both through the existing millisecond timestamp rules and matching-session clear semantics.
- [A cache file is truncated or maliciously large] → Use atomic writes, byte limits, strict JSON validation, and per-file failure isolation.
- [OS process IDs are eventually reused] → Combine the PID namespace with pane-manifest cleanup and current timestamps; stale cache is never accepted for nonexistent panes. Full resurrection identity is explicitly outside this change.
- [All sidebar instances run the recovery helper] → Accept small redundant reads to avoid leader-election races; no helper writes occur during snapshot reads.

## Migration Plan

Install `status_store.py` beside the two existing global hooks, install the updated bridges, approve `RunCommands`, then deploy or start the updated plugin. Existing in-memory status is written into plugin cache after its next state change; the host journal begins with the next Codex lifecycle or notification event. Starting a prompt is sufficient to seed both layers.

Rollback restores the previous WASM and bridges. Journal and plugin-cache files are inert and may be removed manually; the external version-1 pipe protocol remains compatible.

## Open Questions

None. The implementation deliberately targets detach/reattach of a live Zellij server and treats full session resurrection as a separate capability.
