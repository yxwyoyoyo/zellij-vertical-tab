## Why

The dev layout currently sizes the sidebar with `size="13%"`, which varies with the viewport width. At 13% of a typical fullscreen terminal the sidebar lands near 32 columns, but the actual width drifts with the window — narrower on a tiled half-screen, wider on an ultrawide display. Setting a fixed 32-column width gives consistent sidebar geometry regardless of viewport size.

## What Changes

- Replace the dev layout's percentage sidebar width (`size="13%"`) with a fixed column count (`size=32`).
- Update the layout-integration spec to describe a fixed rather than percentage-based initial width.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: The development layout sidebar width is now a fixed 32 columns instead of a viewport-dependent percentage.

## Impact

- `zellij.kdl` development layout.
- `openspec/specs/vertical-tab-sidebar/spec.md` layout-integration requirement.
- No plugin code, hook, permission, dependency, or Zellij ABI change.
