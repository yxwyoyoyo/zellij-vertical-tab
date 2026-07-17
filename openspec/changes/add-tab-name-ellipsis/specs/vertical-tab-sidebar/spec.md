## MODIFIED Requirements

### Requirement: Width-fitted rows
Every rendered tab row SHALL fit the sidebar's current content width and SHALL fill that width so selected styling spans the complete row. When an aggregate agent status exists, the row SHALL reserve a right-aligned suffix for its glyph and optional pane count without displaying an agent-name prefix. When a tab name exceeds its remaining cell budget, the row SHALL end the visible name portion with a single-cell ellipsis.

#### Scenario: Tab name is short
- **WHEN** the formatted position, complete tab name, and optional status badge fit within the available width
- **THEN** the plugin renders the complete name without an ellipsis
- **AND** pads the row to the available width

#### Scenario: Tab name is long
- **WHEN** the complete tab name exceeds the cells remaining after the position prefix and optional status badge are reserved
- **THEN** the plugin truncates the tab-name portion on a character boundary
- **AND** appends a single-cell `…` ellipsis within the name budget
- **AND** preserves the complete right-aligned badge when one exists

#### Scenario: Long tab name contains wide characters
- **WHEN** a long tab name contains characters occupying more than one terminal cell
- **THEN** the plugin measures and truncates the name by terminal cells without splitting a character
- **AND** the rendered row still occupies exactly the available width

#### Scenario: No cell remains for a tab name
- **WHEN** the position prefix or reserved badge leaves no cell for the tab name
- **THEN** the plugin omits the name and ellipsis rather than displacing the reserved content

#### Scenario: Session has ten or more tabs
- **WHEN** tab positions require multiple digits
- **THEN** the plugin right-aligns all displayed positions to the width of the largest position

#### Scenario: Tab has no agent status
- **WHEN** no tracked agent pane belongs to a tab
- **THEN** the row applies the same ellipsis behavior using all cells remaining after the position prefix

#### Scenario: Active tab has an agent status
- **WHEN** the active tab row contains a status badge
- **THEN** selected styling spans the complete fitted row including the badge
