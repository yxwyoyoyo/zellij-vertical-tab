## Why

Codex and Claude Code currently duplicate payload construction, timestamps, Zellij publication, persistence ordering, and snapshot handling. The durable store is also kept under `hooks/codex/` even though both agents depend on it. That makes the next agent integration copy an existing bridge instead of implementing a small, explicit adapter contract.

## What Changes

- Add a dependency-free common Python bridge runtime with a normalized agent-update interface.
- Move the durable status store and its tests from the Codex directory into `hooks/common/`.
- Keep Codex and Claude Code entrypoints as thin native-event adapters that delegate payload construction, persistence, publication, and snapshots to the common runtime.
- Keep agent-specific behavior in its adapter: Codex transcript review and process-exit watching, Codex external-notifier forwarding, and Claude terminal-sequence responses.
- Update installation, test, development, and OpenWiki documentation for the common adapter contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: define the common adapter interface and dependency-free installation contract used by every supported agent.

## Impact

- `hooks/common/`: normalized bridge runtime, durable store, and contract tests.
- `hooks/codex/` and `hooks/claude/`: agent-specific mappings and extensions only.
- `mise.toml`, README, DEVELOPMENT, OpenWiki, and OpenSpec: common test and installation workflow.
