## Why

Agent status integration currently scales poorly with tab and session count: an untargeted Zellij pipe is rebroadcast between every sidebar instance, recurring Codex session-start hooks can create duplicate 25 MB exit watchers, and persistent cache discovery scans a shared directory that grows across servers. Hook process startup also adds measurable latency to every tool boundary.

## What Changes

- Ensure at most one exit watcher runs for a Codex process and use event-driven process-exit observation where the platform supports it.
- Stop forwarding lifecycle messages that Zellij already broadcasts, and elect one sidebar instance to publish cross-instance focus and acknowledgement observations.
- Walk the process tree only once during session start and keep the bounded automatic-review transcript lookup.
- Store plugin cache snapshots under a server-scoped directory, migrate current legacy snapshots, and prune host-journal directories for dead Zellij servers.
- Add scale-oriented tests and document the bounded performance model.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Bound lifecycle watcher count, cross-instance message amplification, hook work, and persistent recovery discovery while preserving status semantics.

## Impact

- `hooks/codex/`: watcher ownership, process discovery, journal cleanup, and Python tests.
- `src/main.rs`: external-pipe handling, synchronization leadership, server-sharded cache migration, and Rust tests.
- OpenSpec and maintainer documentation: performance invariants and verification guidance.
