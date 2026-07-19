## ADDED Requirements

### Requirement: Zellij-native hierarchy presentation
The sidebar SHALL render its visible tab and pane hierarchy using Zellij's native nested-list presentation and active theme rather than hard-coded colors or hand-built hierarchy spacing.

#### Scenario: Tab row is rendered
- **WHEN** a visible row represents a tab
- **THEN** the row uses a top-level native list item with Zellij's tab-level bulletin, typography, and list styling
- **AND** its fitted name, optional overflow indicator, and optional status badge remain visible within the available content width
- **AND** the row does not repeat its one-based tab position after the native bulletin

#### Scenario: Pane child row is rendered
- **WHEN** a visible row represents a terminal pane in a multi-pane tab
- **THEN** the row uses a level-one native list item with Zellij's child bulletin, indentation, typography, and list styling
- **AND** its fitted pane title and optional status badge remain visible within the available content width

#### Scenario: Active row is rendered
- **WHEN** a row represents the active tab or focused pane child
- **THEN** it uses Zellij's native selected-list styling across the complete sidebar width
- **AND** any state-colored status badge retains its distinct theme-derived foreground treatment

#### Scenario: Inactive row is rendered
- **WHEN** a row represents neither the active tab nor the focused pane child
- **THEN** it uses Zellij's native unselected-list styling

#### Scenario: Sidebar is narrow
- **WHEN** native list chrome, the row prefix, and an optional status badge leave limited content width
- **THEN** the row preserves required chrome, prefix, and badge before truncating the name by terminal cells with an ellipsis
- **AND** the rendered row does not exceed the sidebar width

## MODIFIED Requirements

### Requirement: Ordered vertical tab list
The plugin SHALL render session tabs as a vertical list in Zellij tab-position order, with at most one tab per sidebar row. The native top-level bulletin SHALL be the sole persistent leading marker; the plugin SHALL NOT repeat the tab's one-based position in its visible label.

#### Scenario: Tabs are available
- **WHEN** Zellij supplies a tab-state update
- **THEN** each visible row shows the tab's name without a displayed tab position
- **AND** the row order matches the order supplied by Zellij

#### Scenario: No tabs or no drawable area
- **WHEN** the tab list is empty or the sidebar has zero rows or columns
- **THEN** the plugin renders no tab rows

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
- **THEN** the plugin does not display or reserve content cells for those positions
- **AND** reserves the native component's top-level or child chrome before fitting each label

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
