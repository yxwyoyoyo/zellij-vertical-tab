## 1. Pane-aware row model

- [x] 1.1 Retain terminal pane metadata from `PaneUpdate` while preserving pane lifecycle cleanup and peer discovery
- [x] 1.2 Build a deterministic flattened tab-and-pane row model with child rows only for tabs containing multiple terminal panes
- [x] 1.3 Resolve compact one-pane and per-child multi-pane agent badge ownership from pane-keyed records

## 2. Rendering and interaction

- [x] 2.1 Generalize terminal-cell-aware row fitting and ellipsis for aligned tab names and indented pane titles
- [x] 2.2 Render active tabs and focused pane children with complete-row styling and state-colored badges
- [x] 2.3 Apply overflow indicators and bounded scrolling to flattened hierarchy rows
- [x] 2.4 Dispatch tab-row clicks to tab switching and pane-row clicks to exact terminal-pane focus
- [x] 2.5 Reduce pane-child indentation to one cell beyond the tab-name column and update formatting coverage
- [x] 2.6 Add one trailing padding cell after status badges and keep badge coloring scoped to the icon
- [x] 2.7 Reserve one trailing padding cell on rows without agent status and cover narrow-width behavior

## 3. Verification and delivery

- [x] 3.1 Add host tests for pane filtering/order, adaptive hierarchy, badge placement, formatting, focus selection, scrolling, and click targets
- [x] 3.2 Update user documentation for adaptive pane rows, per-pane badges, ordering, and mouse behavior
- [x] 3.3 Run formatting, host tests, clippy, release WASM build, and strict OpenSpec validation
- [x] 3.4 Install the release WASM and verify adaptive pane rendering in a live or headless Zellij session
