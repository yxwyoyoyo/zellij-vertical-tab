## ADDED Requirements

### Requirement: Native bell attention presentation
The sidebar SHALL render Zellij's persistent bell notification as a distinct theme-colored Nerd Font bell on the owning tab row while retaining exact pane-owned Codex lifecycle badges.

#### Scenario: A one-pane tab has status and bell attention
- **WHEN** a tab has one terminal pane with Codex state and Zellij reports `has_bell_notification`
- **THEN** its compact tab row displays the pane status followed by the bell icon ``
- **AND** each icon retains its distinct theme-derived foreground style

#### Scenario: A multi-pane tab has bell attention
- **WHEN** a tab has multiple terminal panes and Zellij reports `has_bell_notification`
- **THEN** the parent tab row displays the bell icon
- **AND** each child pane row continues to display only its own Codex lifecycle status

#### Scenario: Bell attention is acknowledged
- **WHEN** Zellij clears `has_bell_notification` after the tab receives attention
- **THEN** the sidebar removes the bell icon without changing any Codex lifecycle status

#### Scenario: No bell attention exists
- **WHEN** Zellij reports no persistent bell for a tab
- **THEN** the sidebar does not display an attention icon for that tab

#### Scenario: The sidebar is too narrow for both suffixes
- **WHEN** a compact tab row has status and bell attention but cannot preserve both icons and their separator
- **THEN** the sidebar preserves the bell icon and omits the status icon at that width

## MODIFIED Requirements

### Requirement: Width-fitted rows
Every rendered tab or pane row SHALL fit the sidebar's current content width and SHALL fill that width so selected styling spans the complete row. Every row SHALL reserve one trailing padding cell when the available width can still preserve its required prefix, status badge, or attention badge. A row with agent status or native bell attention SHALL reserve a right-aligned suffix without displaying an agent-name prefix. When a tab name or pane title exceeds its remaining cell budget, the row SHALL end the visible name portion with a single-cell ellipsis.

#### Scenario: Tab name or pane title is short
- **WHEN** the complete row prefix, name, and optional status and attention badges fit within the available width
- **THEN** the plugin renders the complete name without an ellipsis
- **AND** pads the row to the available width

#### Scenario: Tab name or pane title is long
- **WHEN** the complete name exceeds the cells remaining after the row prefix and optional status and attention badges are reserved
- **THEN** the plugin truncates the name portion on a character boundary
- **AND** appends a single-cell `…` ellipsis within the name budget
- **AND** preserves the complete right-aligned suffix when it fits

#### Scenario: Long name contains wide characters
- **WHEN** a long tab name or pane title contains characters occupying more than one terminal cell
- **THEN** the plugin measures and truncates the name by terminal cells without splitting a character
- **AND** the rendered row still occupies exactly the available width

#### Scenario: No cell remains for a name
- **WHEN** the row prefix or reserved suffix leaves no cell for its tab name or pane title
- **THEN** the plugin omits the name and ellipsis rather than displacing the reserved content

#### Scenario: Session has ten or more tabs
- **WHEN** tab positions require multiple digits
- **THEN** the plugin does not display or reserve content cells for those positions
- **AND** reserves the native component's top-level or child chrome before fitting each label

#### Scenario: Row has no agent status
- **WHEN** no agent status or native bell attention belongs on a rendered tab or pane row
- **THEN** the row reserves one trailing padding cell when its complete prefix still fits
- **AND** applies the same ellipsis behavior within the cells remaining between its prefix and trailing padding

#### Scenario: Selected row has an agent status
- **WHEN** an active tab row or focused pane row contains a status or attention badge
- **THEN** selected styling spans the complete fitted row including the suffix

#### Scenario: Status badge has edge spacing
- **WHEN** a tab row contains one or both badges and the row is wide enough to contain its suffix plus padding
- **THEN** the plugin renders one uncolored padding cell after the complete suffix
