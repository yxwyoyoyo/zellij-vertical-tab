# Vertical Tab Sidebar Specification

## Purpose

Define the user-visible behavior, interaction model, layout integration, and runtime safety constraints of the Zellij vertical tab sidebar.

## Requirements

### Requirement: Ordered vertical tab list
The plugin SHALL render session tabs as a vertical list in Zellij tab-position order, with at most one tab per sidebar row.

#### Scenario: Tabs are available
- **WHEN** Zellij supplies a tab-state update
- **THEN** each visible row shows the tab's one-based position followed by its name
- **AND** the row order matches the order supplied by Zellij

#### Scenario: No tabs or no drawable area
- **WHEN** the tab list is empty or the sidebar has zero rows or columns
- **THEN** the plugin renders no tab rows

### Requirement: Active tab presentation
The plugin SHALL visually distinguish the active tab using Zellij's selected text styling and SHALL keep the active tab inside the visible sidebar window.

#### Scenario: Active tab changes outside the sidebar
- **WHEN** a keyboard command or another Zellij action activates a tab outside the current visible window
- **THEN** the plugin moves the window by the minimum amount needed to reveal that tab

#### Scenario: Active tab is already visible
- **WHEN** the active tab changes but remains inside the current visible window
- **THEN** the plugin preserves the current scroll position

### Requirement: Adaptive pane hierarchy
The plugin SHALL represent every tab with a parent row and SHALL add permanently visible indented terminal-pane child rows only when that tab contains more than one terminal pane. The plugin SHALL NOT render an expand/collapse icon or maintain show/hide state.

#### Scenario: Tab has no terminal pane
- **WHEN** Zellij reports no terminal pane for a tab
- **THEN** the plugin renders only the tab row

#### Scenario: Tab has one terminal pane
- **WHEN** Zellij reports exactly one terminal pane for a tab
- **THEN** the plugin renders only the tab row
- **AND** does not render a redundant pane child row

#### Scenario: Tab has multiple terminal panes
- **WHEN** Zellij reports two or more terminal panes for a tab
- **THEN** the plugin renders the tab row followed immediately by one indented child row for every terminal pane
- **AND** renders no show/hide control

#### Scenario: Pane types are mixed
- **WHEN** a tab contains tiled, floating, suppressed, or plugin panes
- **THEN** the plugin includes all terminal panes as children when the terminal-pane count exceeds one
- **AND** excludes plugin panes from the child rows and terminal-pane count

#### Scenario: Pane children are ordered
- **WHEN** the plugin renders multiple terminal-pane children
- **THEN** tiled panes precede floating panes and floating panes precede suppressed panes
- **AND** panes within each layer are ordered by vertical position, horizontal position, and pane ID

### Requirement: Focused pane presentation
The plugin SHALL visually distinguish a focused terminal-pane child row using Zellij selected text styling when its owning multi-pane tab is active.

#### Scenario: Child pane is focused
- **WHEN** a terminal pane is focused in the active tab and its pane child row is rendered
- **THEN** that child row receives selected styling across its complete fitted width

#### Scenario: Child pane is not focused
- **WHEN** a pane child row does not represent the focused terminal pane of the active tab
- **THEN** the plugin does not apply focused-pane selected styling to that child row

### Requirement: Width-fitted rows
Every rendered tab or pane row SHALL fit the sidebar's current content width and SHALL fill that width so selected styling spans the complete row. Every row SHALL reserve one trailing padding cell when the available width can still preserve its required prefix or status badge. A row with agent status SHALL reserve a right-aligned suffix for its glyph without displaying an agent-name prefix. When a tab name or pane title exceeds its remaining cell budget, the row SHALL end the visible name portion with a single-cell ellipsis.

#### Scenario: Tab name or pane title is short
- **WHEN** the complete row prefix, name, and optional status badge fit within the available width
- **THEN** the plugin renders the complete name without an ellipsis
- **AND** pads the row to the available width

#### Scenario: Tab name or pane title is long
- **WHEN** the complete name exceeds the cells remaining after the row prefix and optional status badge are reserved
- **THEN** the plugin truncates the name portion on a character boundary
- **AND** appends a single-cell `…` ellipsis within the name budget
- **AND** preserves the complete right-aligned badge when one exists

#### Scenario: Long name contains wide characters
- **WHEN** a long tab name or pane title contains characters occupying more than one terminal cell
- **THEN** the plugin measures and truncates the name by terminal cells without splitting a character
- **AND** the rendered row still occupies exactly the available width

#### Scenario: No cell remains for a name
- **WHEN** the row prefix or reserved badge leaves no cell for its tab name or pane title
- **THEN** the plugin omits the name and ellipsis rather than displacing the reserved content

#### Scenario: Session has ten or more tabs
- **WHEN** tab positions require multiple digits
- **THEN** the plugin right-aligns all displayed positions to the width of the largest position
- **AND** indents pane child titles one cell beyond the tab-name column

#### Scenario: Row has no agent status
- **WHEN** no agent status belongs on a rendered tab or pane row
- **THEN** the row reserves one trailing padding cell when its complete prefix still fits
- **AND** applies the same ellipsis behavior within the cells remaining between its prefix and trailing padding

#### Scenario: Selected row has an agent status
- **WHEN** an active tab row or focused pane row contains a status badge
- **THEN** selected styling spans the complete fitted row including the badge

#### Scenario: Status badge has edge spacing
- **WHEN** a tab or pane row contains a status badge and the row is wide enough to contain it plus padding
- **THEN** the plugin renders one uncolored padding cell after the complete badge

### Requirement: Overflow navigation
The plugin SHALL support bounded vertical navigation when the flattened tab-and-pane row count exceeds the available sidebar rows.

#### Scenario: Hierarchy rows exist above the visible window
- **WHEN** the first displayed row is not the first flattened hierarchy row
- **THEN** the first displayed row uses an upward overflow indicator

#### Scenario: Hierarchy rows exist below the visible window
- **WHEN** the last displayed row is not the last flattened hierarchy row
- **THEN** the last displayed row uses a downward overflow indicator

#### Scenario: User scrolls the sidebar
- **WHEN** the user scrolls up or down over the sidebar
- **THEN** the visible window moves by one flattened hierarchy row without passing its valid bounds

### Requirement: Mouse tab switching
The plugin SHALL activate the target represented by a valid left-clicked sidebar row.

#### Scenario: User clicks a tab row
- **WHEN** the user left-clicks a rendered tab row
- **THEN** the plugin asks Zellij to switch to that tab using its one-based position

#### Scenario: User clicks a pane child row
- **WHEN** the user left-clicks a rendered terminal-pane child row
- **THEN** the plugin asks Zellij to focus that exact terminal pane, switching to its tab and layer as needed

#### Scenario: User clicks outside the rendered rows
- **WHEN** the user left-clicks a row that does not map to a flattened hierarchy row
- **THEN** the plugin performs no tab-switching or pane-focusing action

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

### Requirement: Startup-safe unselectable sidebar
The sidebar SHALL become unselectable only after the plugin receives its first event, while remaining able to receive mouse events.

#### Scenario: Plugin is loaded in the default tab template
- **WHEN** Zellij invokes the plugin's load lifecycle method
- **THEN** the plugin does not call `set_selectable(false)` during that method
- **AND** it calls `set_selectable(false)` once when processing its first event

#### Scenario: Layout wraps normal children
- **WHEN** the sidebar is an unselectable sibling in `default_tab_template`
- **THEN** the layout wraps `children` inside a separate `pane` block

### Requirement: Zellij plugin compatibility
The built plugin SHALL use the Zellij plugin ABI expected by the installed Zellij binary and SHALL expose the command-module entrypoint required by Zellij.

#### Scenario: Plugin is built for Zellij
- **WHEN** the release artifact is produced
- **THEN** it targets `wasm32-wasip1`
- **AND** it is built as a binary crate that exports `_start`
- **AND** its `zellij-tile` version matches the Zellij binary version
