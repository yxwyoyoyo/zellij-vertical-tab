## Why

Codex status badges show what each session is doing, but they do not actively draw attention when an answer is ready or approval is required. Codex and Zellij already share a native BEL notification path, so the sidebar can expose that signal without inventing a second acknowledgement lifecycle.

## What Changes

- Configure Codex TUI notifications for completed turns and approval requests to always emit BEL, allowing Zellij to decide whether the affected tab flashes or retains attention.
- Render Zellij's persistent tab bell state as a theme-colored Nerd Font bell icon on the corresponding tab row.
- Keep exact Codex lifecycle status on its existing compact tab row or owning pane child; in multi-pane tabs the attention bell remains on the parent because Zellij exposes bell persistence per tab rather than per pane.
- Preserve status and attention icons under truncation, native selected styling, and narrow sidebar widths.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Codex answer-ready and approval-required events request native terminal attention, which Zellij retains when their tab is inactive.
- `vertical-tab-sidebar`: Tab rows display Zellij's persistent bell notification state without changing pane-specific status ownership.

## Impact

- `src/main.rs` tab-row model, suffix formatting, semantic styling, and unit tests.
- User-level `~/.codex/config.toml` notification settings.
- README and generated OpenWiki documentation.
- No new plugin permission, status protocol field, dependency, or Zellij ABI change.
