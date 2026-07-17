## Why

An aggregate tab badge hides which Codex pane needs attention when a tab contains multiple terminal panes. The sidebar should expose per-pane identity and status only when that extra detail is useful, while keeping the common one-pane tab compact.

## What Changes

- Keep tabs with zero or one terminal pane as a single existing-style tab row.
- Render terminal panes as permanently visible indented child rows beneath tabs that contain multiple panes, without expand/collapse controls.
- Move agent status from the parent tab to the owning child pane for multi-pane tabs; retain the status on the tab row for a one-pane tab.
- Order pane children deterministically from their Zellij layer and screen geometry, and mark the focused pane distinctly.
- Allow a pane child row to focus its exact terminal pane when clicked.
- Apply bounded scrolling and terminal-cell-aware ellipsis to the flattened tab-and-pane row list.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: Expand multi-pane tabs into pane child rows and extend row navigation, fitting, focus presentation, and mouse behavior to the flattened hierarchy.
- `agent-status`: Render per-pane badges on multi-pane child rows while preserving the compact one-pane tab badge.

## Impact

- `src/main.rs`: pane manifest storage, row modeling, rendering, scrolling, selected/focused styling, and click dispatch.
- Host unit tests: pane ordering, hierarchy flattening, badge ownership, row fitting, and click targets.
- Runtime permissions and the status transport protocol remain unchanged; the existing Zellij 0.44.3 pane metadata and focus command are used.
