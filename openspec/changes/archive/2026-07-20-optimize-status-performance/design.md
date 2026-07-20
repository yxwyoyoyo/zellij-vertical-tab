## Context

The layout creates one sidebar plugin instance per tab. Zellij documents that an untargeted named CLI pipe is delivered to every listening plugin, but each instance currently forwards the same lifecycle update to every peer. Focus changes are also broadcast by every instance. These paths create quadratic plugin-message traffic as tab count grows.

Every Codex `SessionStart` invokes the Python bridge. The event can represent startup, resume, clear, or compact, and every invocation currently starts a detached Python process that polls its Codex ancestor twice per second. A measured watcher uses about 24.7 MB RSS. The hook also repeats process-tree discovery during session start.

Plugin cache snapshots are flat files keyed by server and plugin ID under one shared directory. Reads filter by server only after scanning every entry, so historical servers and plugin IDs create unbounded discovery work. The host journal is already server-sharded but does not remove directories after a server exits.

## Goals / Non-Goals

**Goals:**

- Keep no more than one long-lived exit watcher per Codex process.
- Reduce steady-state lifecycle and focus synchronization from quadratic to linear message traffic.
- Avoid repeated process-tree walks within one hook invocation.
- Bound current-server cache discovery independently of historical server count.
- Preserve detached recovery, late-joining peer synchronization, focus acknowledgement, and lifecycle ordering.

**Non-Goals:**

- Do not replace the Python bridge with Rust in this change.
- Do not change badge appearance or lifecycle meaning.
- Do not delete cache state belonging to a process that may still be alive.
- Do not introduce a resident shared daemon.

## Decisions

### Deduplicate watchers with an advisory process lock

The watcher acquires a non-blocking lock keyed by Zellij server PID and Codex PID and holds it for its lifetime. A duplicate detached process exits immediately. PID reuse is safe because the kernel releases the lock when its owner exits; the small lock file may remain and be reacquired.

On platforms with `kqueue`, the watcher registers `NOTE_EXIT` for event-driven termination. Other platforms retain a conservative polling fallback with a longer interval. Resume remains supported because a genuinely new Codex PID obtains a distinct lock.

### Treat the CLI lifecycle pipe as the existing-instance broadcast

An `AGENT_STATUS_PIPE` recipient applies the update locally and does not relay it. Zellij already delivers that message to every listening instance. Peer snapshot request/response remains for a sidebar created after the lifecycle event.

For focus and acknowledgement messages, the live instance with the lowest plugin ID is the synchronization leader. Every instance still computes local state. A nonleader that observes a focus transition reports it once to the leader, while only the leader fans the observation or resulting acknowledgement out to peers. This stays linear even when the elected leader is not the sidebar instance that first receives the transition. When the leader closes, the next-lowest live ID becomes leader from the next pane manifest.

### Combine ancestry discovery during session start

The lifecycle bridge walks its ancestor chain once and captures both the nearest Codex PID and Zellij server PID. Other events retain the same bounded walk because the server PID is required to isolate the durable journal. The Zellij CLI process remains necessary to publish the pipe.

### Shard plugin cache by server and migrate lazily

New snapshots live under `/cache/agent-status-<zellij-pid>/agent-status-<plugin-id>.json`. Restore scans only that directory. If it does not exist, the first instance reads compatible flat files for the current server, creates the shard, and persists the merged snapshot; later instances avoid the legacy directory scan.

Host journal cleanup inspects only numeric server directories and removes one only when `kill(pid, 0)` proves that PID is absent. Permission errors are treated as alive. This bounds dead journal accumulation without touching a possibly live session.

## Risks / Trade-offs

- [Synchronization leader misses an application event] → A nonleader reports its observation once to the leader, the leader performs the only fanout, peer snapshots retain recovery for late joiners, and tests cover leader turnover.
- [Filesystem does not support advisory locks] → Skip the best-effort watcher rather than create unbounded helpers; normal pane cleanup and later lifecycle replacement remain available.
- [Kqueue registration races process exit] → Check liveness before registration and treat `ESRCH` as completion; use polling for unsupported errors.
- [Legacy cache migration occurs during concurrent plugin startup] → Atomic snapshot writes and merge ordering remain authoritative; creating the server shard makes subsequent restores bounded.
- [PID is reused before journal cleanup] → A live PID is retained, favoring stale storage over deletion of potentially active state.

## Migration Plan

Deploy hooks and WASM together. Existing flat cache files remain readable for the current server and are migrated on first restore. New writes use server shards. Rollback leaves version-1 payloads and legacy files readable; older builds ignore the new cache subdirectories.

## Open Questions

None.
