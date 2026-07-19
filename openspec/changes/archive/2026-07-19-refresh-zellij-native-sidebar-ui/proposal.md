## Why

The sidebar currently imitates a plain text list, so its active rows, hierarchy, and spacing feel visually separate from Zellij's built-in UI. Reusing Zellij's native nested-list component will make the sidebar inherit the same theme surfaces, bullets, indentation, and selected-row treatment as the rest of the application.

## What Changes

- Render the visible tab and pane hierarchy through Zellij's native nested-list UI component.
- Use native top-level and child bullets, indentation, bold labels, and selected/unselected list styles instead of hand-built blank prefixes and full-row text selection.
- Remove the displayed one-based tab number because the native `>` bulletin already identifies every tab row; internal tab order and click targets remain unchanged.
- Preserve right-aligned, theme-colored agent badges and cell-aware ellipsis inside the space left by native list chrome.
- Preserve row ordering, overflow markers, mouse targets, scrolling, and compact one-pane tabs.
- Verify the result against Zellij 0.44.3 in a live plugin reload and at narrow and wide sidebar widths.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: The sidebar hierarchy and selected rows adopt Zellij's native nested-list presentation while retaining existing interaction and width-fitting behavior.

## Impact

- `src/main.rs` rendering and pure row-formatting helpers/tests.
- User-visible screenshots/examples and OpenWiki rendering documentation.
- No status protocol, layout geometry, hook, permission, dependency, or Zellij ABI change.
