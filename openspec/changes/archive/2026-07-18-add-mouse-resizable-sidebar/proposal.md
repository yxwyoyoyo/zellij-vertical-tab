## Why

The sidebar is locked to a fixed 32-column dimension, so Zellij rejects native pane-resize operations even though version 0.44.3 supports dragging tiled pane borders. Users should be able to adjust the sidebar interactively without giving the plugin focus or restarting for every width experiment.

## What Changes

- Replace the fixed sidebar dimension with a flexible percentage dimension that starts near 32 columns on the current terminal.
- Expose the native tiled boundary between the sidebar and content as the mouse-resize handle while keeping the sidebar plugin itself borderless and unselectable.
- Preserve the user's pane-frame preference; visible pane frames improve discoverability but are not required for resizing.
- Document that native resize is per tab/session and that new tabs begin from the layout percentage.
- Verify actual border dragging in a fresh disposable Zellij session rather than relying on hot reload.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: The session layout becomes flexibly resizable through Zellij's native tiled-pane mouse interaction instead of enforcing a fixed 32-column width.

## Impact

- Project and installed Zellij layout KDL.
- Zellij mouse handling and pane-boundary behavior.
- Sidebar layout documentation and development/runtime verification guidance.
- No status protocol, Rust dependency, or plugin ABI change.
