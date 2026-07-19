## Context

The plugin already tracks Codex lifecycle state per terminal pane. Codex can also emit TUI notifications for `agent-turn-complete` and `approval-requested` using BEL. Zellij 0.44.3 turns BEL into a transient visual flash and exposes persistent `has_bell_notification` and transient `is_flashing_bell` fields on `TabInfo`.

Zellij's public pane model does not expose which terminal pane emitted a retained bell. A sidebar that manufactures pane-level bell ownership would therefore become incorrect when several panes share a tab.

## Goals / Non-Goals

**Goals:**

- Draw attention to completed answers and approval requests without stealing focus.
- Preserve Zellij's acknowledgement and clearing behavior.
- Keep Codex status ownership exact for multi-pane tabs.
- Keep both status and attention visible and theme-aligned when width permits.

**Non-Goals:**

- Show macOS Notification Center banners.
- Infer whether a completed answer contains a question for the user.
- Attribute Zellij's tab-scoped retained bell to a specific pane.
- Add a custom acknowledgement store or change the status transport protocol.

## Decisions

### Use Codex BEL notifications

Set `tui.notifications` to completed turns and approval requests, `tui.notification_method` to `bel`, and `tui.notification_condition` to `always`. This uses Codex's structured notification events and Zellij's native visual-bell path. `always` is required because moving between Zellij tabs or panes does not reliably change the terminal-level focus state observed by Codex. Zellij then owns whether an active tab flashes or an inactive tab retains attention.

### Treat Zellij's tab bell as authoritative

Render an attention icon exactly when `TabInfo.has_bell_notification` is true. Do not retain another plugin-local attention bit: Zellij already owns persistence and acknowledgement, while duplicated state would be lost during reload and could disagree across clients.

### Keep bell and agent status at their precise scopes

For a one-pane tab, the compact row may show the pane's status icon followed by the tab attention bell. For a multi-pane tab, the parent tab shows the bell and child rows retain their individual Codex state. This reflects the strongest ownership guarantees offered by each source.

### Reuse native theme ranges and Nerd Font metrics

Use the Nerd Font bell `` and Zellij emphasis level 0 for attention. The existing lifecycle state retains its own semantic color. Both ranges are applied before selected-row styling so the native selected background remains intact.

### Prioritize attention at extreme widths

Reserve the complete `status bell` suffix when it fits. If the content budget cannot hold both icons and their separator, render the attention bell alone rather than clipping it. Names continue to ellipsize before reserved suffixes.

## Risks / Trade-offs

- [Bell ownership is tab-scoped] → Keep the bell on the parent row and retain exact state on pane children.
- [Existing Codex sessions do not reread TUI configuration] → Start a new Codex session for live verification.
- [A terminal profile lacks Nerd Font glyphs] → Keep the documented Nerd Font Mono prerequisite and single-cell tests.
- [Narrow rows cannot show every suffix] → Prioritize the attention signal and restore full status at normal widths.

## Migration Plan

1. Add explicit user-level Codex TUI notification settings.
2. Render and test `TabInfo.has_bell_notification` in the sidebar.
3. Build and hot-reload the plugin; use a new Codex session for notification verification.
4. Roll back by removing the three Codex TUI settings and the attention suffix; no persisted plugin data requires migration.

## Open Questions

- Whether macOS Notification Center banners should be offered later as an optional, separate notifier integration.
