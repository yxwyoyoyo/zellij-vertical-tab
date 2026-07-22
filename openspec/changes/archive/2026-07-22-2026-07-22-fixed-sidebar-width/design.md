## Context

Zellij layout pane sizes accept either a percentage string (`"13%"`) or a bare integer for fixed columns/rows. The dev layout has used a percentage since the sidebar was introduced, targeting roughly 32 columns at common viewport widths. In practice, this percentage varies the sidebar width with the terminal window, which can make the sidebar too narrow on tiled displays or unnecessarily wide on large screens.

A fixed 32-column width gives the sidebar enough room for tab names, native list chrome, and status badges while being narrow enough to leave most of the screen for content.

## Goals / Non-Goals

**Goals:**

- Make the sidebar width predictable and consistent across viewport sizes.
- Keep the sidebar wide enough for typical tab names with native list chrome and badges.

**Non-Goals:**

- Make the sidebar width user-configurable.
- Change the resize behavior (the boundary remains draggable).
- Change any plugin rendering or layout logic.

## Decisions

### Use a fixed 32-column width

Replace `size="13%"` with `size=32` in the dev layout. This is already the rough target width the percentage was chosen to produce, so the visual outcome is the same at the tested viewport size while being stable everywhere else.

Alternative: keep the percentage and document the expected viewport. Rejected because a fixed width directly expresses the intent and avoids the viewport dependency.

### No new configuration surface

The layout file is the single source of truth. Users who want a different width can edit their local copy of `zellij.kdl`.

## Risks / Trade-offs

- [Very narrow terminals (< ~50 columns) may show a cramped content area] → Acceptable: the sidebar is resizable via the draggable boundary, and terminals that narrow are uncommon for development work.
- [Very wide terminals may leave the sidebar looking small relative to content] → Acceptable: the user can resize it, and the fixed width ensures it never exceeds what the plugin needs.

## Migration Plan

1. Edit `size="13%"` → `size=32` in `zellij.kdl`.
2. Update the layout-integration spec wording from "percentage" to "fixed".
3. Verify in a fresh Zellij session.
4. No stored state, configuration, or installation migration required.

## Open Questions

None.
