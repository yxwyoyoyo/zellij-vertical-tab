## Why

Tabs that run Codex currently look identical to ordinary tabs, so a user must visit each pane to learn whether an agent is working, waiting for approval, or finished. The vertical sidebar already provides a session-wide overview and is the natural place to expose this state, including when several Codex panes run concurrently.

## What Changes

- Add globally installable Codex lifecycle and completion-notifier bridges that publish state to the running Zellij session from any working directory.
- Distinguish a delivered answer (`done`) from an untouched session (`idle`) and clear the badge when Codex exits.
- Track status per terminal pane and current Codex session inside the plugin.
- Associate terminal panes with their owning tabs through Zellij pane updates.
- Aggregate concurrent Codex panes per tab and render a prefix-free, right-aligned status badge and count.
- Remove state when panes disappear, replace stale sessions in reused panes, and reject out-of-order messages.
- Preserve existing tab switching, scrolling, active-tab styling, and the 24-column layout.

## Capabilities

### New Capabilities
- `agent-status`: Codex lifecycle transport, per-pane status tracking, multi-pane aggregation, and cleanup behavior.

### Modified Capabilities
- `vertical-tab-sidebar`: Tab rows may reserve a right-aligned suffix for an aggregated status glyph and pane count.

## Impact

- `src/main.rs`: new pipe lifecycle handling, pane manifest state, aggregation, and badge-aware formatting.
- `hooks/codex/`, global installation under `~/.codex`, and the user-level `notify` configuration: translate Codex lifecycle events and completion notifications into Zellij pipe messages for every local Codex project while preserving an existing notifier.
- `Cargo.toml`/`Cargo.lock`: add structured payload parsing and terminal-cell width support.
- Zellij permissions: the plugin additionally requests `ReadCliPipes` and `MessageAndLaunchOtherPlugins`.
- Tests and documentation: cover protocol validation, concurrent panes, stale events, cleanup, formatting, installation, and trust requirements.
