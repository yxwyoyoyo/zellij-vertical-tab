## Context

The current renderer prints every visible row independently with `Text` and constructs hierarchy using spaces. Active tabs and focused panes receive full-row `selected()` styling, but the list has none of the bullets, indentation, typography, or list-specific colors used by Zellij's built-in screens.

Zellij 0.44.3 exposes `NestedListItem` and `print_nested_list_with_coordinates`. The server renders top-level items with `> `, child items with `- `, applies `list_selected` or `list_unselected` theme palettes, bolds labels, and pads selected/opaque rows to the requested width. This is the closest stable public primitive to Zellij's native list UI and needs no new dependency or direct palette access.

## Goals / Non-Goals

**Goals:**

- Make tab and pane hierarchy visually native to Zellij across themes.
- Preserve exact row-to-click mapping, overflow behavior, badge semantics, and cell-safe truncation.
- Keep compact one-pane tabs and pane children for multi-pane tabs.
- Remain usable from narrow sidebar widths through the current resizable layout.

**Non-Goals:**

- Recreate the horizontal tab bar's powerline layout vertically.
- Add a header, session name, new iconography, animation, or configuration surface.
- Change status transport, layout sizing, hooks, or pane selection behavior.

## Decisions

### Render the flattened hierarchy as one native nested list

Convert each visible `SidebarRow` into a `NestedListItem`: tab rows use indentation level 0 and pane rows use level 1. Render the complete visible window with one `print_nested_list_with_coordinates` call. This delegates bullets, indentation, bold typography, padding, and selected/unselected list colors to Zellij.

Alternative: use `print_ribbon_with_coordinates` per tab. Rejected because horizontal ribbons add directional separators but do not express parent/child hierarchy, while native nested lists cover both selection and hierarchy.

### Fit content after reserving native list chrome

Zellij's nested-list renderer consumes three cells before top-level content and five before level-one content. Format names, overflow indicators, and badges inside the remaining content width. Keep the existing one-cell trailing inset inside that content budget and preserve terminal-cell-aware ellipsis.

Alternative: pass full-width strings and let Zellij clip them. Rejected because clipping could split the intended name/badge allocation and silently remove status.

### Use the native tab bulletin as the sole leading marker

Do not repeat the one-based tab position after Zellij's top-level `>` bulletin. The two markers compete visually while conveying the same row boundary. Keep Zellij's native bulletin because it is part of the selected component and theme presentation; preserve tab position internally for ordering, active-tab following, click targets, and `switch_tab_to`.

Alternative: keep the number and suppress `>`. Rejected because `NestedListItem` does not expose bulletin customization, and replacing the native component would also discard the list-specific theme palette and hierarchy treatment selected by this change.

### Apply status colors through `NestedListItem` ranges

Retain the existing state-to-theme-level mapping, but apply the badge range through `NestedListItem::color_range` or `success_color_range` before selection. This keeps state colors theme-derived and lets Zellij combine them with the native selected row background.

### Keep interaction geometry unchanged

The nested list still emits one terminal row per `SidebarRow` in the same order. Mouse targeting and scrolling continue to index the flattened model, so no hit-test or state migration is required.

## Risks / Trade-offs

- [Native list chrome reduces name width by three or five cells] → Fit content against explicit per-level budgets and extend narrow/wide tests.
- [The native component's exact glyphs are controlled by Zellij] → Treat this as intentional theme/version alignment and keep the ABI pinned to 0.44.3.
- [Custom badge ranges could be obscured by selected styling] → Verify all four states on selected and unselected rows after hot reload.
- [A batched component could alter row clearing] → Pass the complete visible height and test shrinking tab/pane sets as well as scrolling.

## Migration Plan

1. Replace per-row text rendering with the native nested-list component.
2. Update pure formatting budgets and tests.
3. Build and hot-reload the plugin, restoring known agent states only when needed for visual verification.
4. Roll back by restoring the previous `Text` row renderer; no stored state or configuration migration is involved.

## Open Questions

None. Native Zellij list presentation is the explicit baseline for the first visual refresh.
