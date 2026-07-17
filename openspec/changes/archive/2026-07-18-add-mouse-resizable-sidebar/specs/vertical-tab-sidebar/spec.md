## MODIFIED Requirements

### Requirement: Session-wide layout integration
The development layout SHALL place a flexibly sized sidebar in every tab, replace the built-in horizontal tab bar, retain the built-in status bar, and expose a native Zellij tiled-pane boundary that can resize the sidebar with the mouse.

#### Scenario: Session starts from the development layout
- **WHEN** Zellij loads `zellij.kdl`
- **THEN** every tab receives a borderless vertical sidebar with an initial percentage width near 32 columns at the tested viewport size
- **AND** normal tab children occupy the sibling content pane
- **AND** the built-in status bar occupies the bottom row

#### Scenario: User drags the sidebar boundary
- **WHEN** Zellij mouse handling is enabled and the user drags the tiled boundary between the sidebar and content horizontally
- **THEN** Zellij resizes the flexible sidebar and sibling content pane continuously with the pointer
- **AND** the sidebar plugin remains unselectable
- **AND** resizing works whether pane frames are displayed or hidden

#### Scenario: User opens another tab
- **WHEN** the user creates a tab after resizing an existing tab's sidebar
- **THEN** the new tab receives the layout's configured initial percentage
- **AND** resizing remains local to the existing tab unless Zellij propagates the geometry itself
