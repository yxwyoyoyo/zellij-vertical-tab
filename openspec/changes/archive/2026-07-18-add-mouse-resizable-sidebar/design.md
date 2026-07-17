## Context

The sidebar currently uses `size=32`, `borderless=true`, and deferred `set_selectable(false)`. Zellij 0.44.3 supports native tiled-pane boundary dragging, but its pane grid rejects resize operations when either side of the boundary has a fixed dimension. When pane frames are hidden, Zellij still reserves and hit-tests a one-cell tiled boundary, so frame visibility does not control resize availability.

The plugin must remain unselectable because that is the startup-safe UI-pane pattern established by previous runtime testing. Zellij's native tiled boundary provides the drag target without making the sidebar plugin selectable or adding plugin-side drag state.

## Goals / Non-Goals

**Goals:**

- Allow horizontal mouse resizing through Zellij's supported tiled-pane interaction.
- Preserve the sidebar's borderless appearance and deferred unselectable lifecycle.
- Start near the current 32-column width on the tested terminal.
- Verify actual pointer-driven geometry changes in a fresh session.

**Non-Goals:**

- Persist a manually resized width across sessions or tabs.
- Add a plugin-rendered resize icon or custom drag protocol.
- Change agent status, row rendering, or pane hierarchy behavior.
- Make a fixed KDL dimension resizable; Zellij explicitly rejects that geometry.

## Decisions

### Use a percentage dimension

Set the sidebar to `size="13%"`. Percentage dimensions are flexible in Zellij's tiled pane grid, while the current integer dimension is fixed and blocks resizing. Thirteen percent starts at approximately 32 columns in the current 245-column terminal.

Alternative: retain `size=32` and call `resize_pane_with_id`. Rejected because Zellij 0.44.3 refuses to resize fixed panes before applying either CLI, plugin API, or mouse strategies.

### Use the native tiled boundary as the handle

Keep the sidebar `borderless=true` and let Zellij's one-cell boundary between the sidebar and sibling content pane serve as the drag handle. With `pane_frames=true`, the boundary is visible as the content pane's left frame; with `pane_frames=false`, the boundary remains present and draggable but is not drawn. `advanced_mouse_actions` controls hover and pane grouping rather than resize dispatch, so it is not a prerequisite. Normal Zellij mouse handling must remain enabled, as it is by default.

Alternative: implement `Mouse::Hold` and `Mouse::Release` inside the plugin. Rejected for the first version because pointer capture across the pane boundary is uncertain, modifier information is not exposed by the plugin mouse enum, and native Zellij already owns border drag semantics.

### Treat resize as tab-local runtime state

The layout percentage is the initial value for every new tab. Zellij owns subsequent geometry, so the plugin does not synchronize widths between sidebar instances or write user configuration.

## Risks / Trade-offs

- [The boundary is less discoverable when pane frames are hidden] → Document its location and mention that pane frames may be enabled as an optional visual aid.
- [Percentage width is not exactly 32 columns on every terminal] → Use 13% as a tested approximation and allow immediate native adjustment.
- [Existing sessions retain their fixed geometry] → Verify in and require a fresh session after migration.
- [A very narrow terminal can produce an undesirable initial width] → Zellij's tiled-pane minimum sizing remains authoritative; users can drag after startup.

## Migration Plan

1. Change project and installed layouts from `size=32` to `size="13%"`.
2. Preserve the user's pane-frame and advanced-mouse preferences.
3. Start a disposable fresh session with hidden pane frames and confirm a pointer drag changes pane geometry.
4. Start normal future sessions with the updated layout; do not restart or destroy the existing `Hub` session automatically.
5. Roll back by restoring `size=32`.

## Open Questions

None.
