## ADDED Requirements

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

## MODIFIED Requirements

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
