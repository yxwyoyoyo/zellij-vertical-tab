## Why

Zellij unloads sidebar plugin runtimes when a client detaches or switches sessions, so their in-memory Codex lifecycle records disappear even though the panes and Codex processes remain alive. Pipes emitted while no client is attached are also lost, which means a plugin-only cache could restore stale status after Codex exits during detachment.

## What Changes

- Persist every Codex lifecycle payload, including `clear` tombstones, in a host-side journal before publishing it to Zellij.
- Cache each sidebar instance's validated lifecycle and acknowledgement snapshot in Zellij's persistent plugin cache.
- Restore plugin cache immediately on load, then reconcile it with the host journal through a validated background-command result.
- Keep timestamp, session-reuse, pane-cleanup, focus-acknowledgement, and peer-synchronization rules unchanged during recovery.
- Treat missing helpers, denied permissions, corrupt files, and malformed recovery output as best-effort failures that do not block Codex or crash the sidebar.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Add durable lifecycle and acknowledgement recovery across Zellij detach, session switch, reattach, and plugin reload boundaries.

## Impact

- `hooks/codex/`: shared host journal, lifecycle/notifier integration, snapshot command, and Python tests.
- `src/main.rs`: persistent plugin snapshots, host snapshot request/result handling, recovery validation, and Rust tests.
- Zellij permissions: add `RunCommands` for the fixed recovery helper invocation.
- Hook installation: install the shared journal module alongside both existing bridges.
- README, OpenWiki, and OpenSpec: document recovery behavior, fallback semantics, permissions, and verification.
