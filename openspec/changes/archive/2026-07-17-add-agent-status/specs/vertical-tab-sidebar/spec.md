## MODIFIED Requirements

### Requirement: Width-fitted rows
Every rendered tab row SHALL fit the sidebar's current content width and SHALL fill that width so selected styling spans the complete row. When an aggregate agent status exists, the row SHALL reserve a right-aligned suffix for its glyph and optional pane count without displaying an agent-name prefix.

#### Scenario: Tab name is short
- **WHEN** the formatted position, tab name, and optional status badge use fewer cells than the available width
- **THEN** the plugin pads the row to the available width

#### Scenario: Tab name is long
- **WHEN** the formatted position, tab name, and optional status badge exceed the available width
- **THEN** the plugin truncates the tab-name portion while preserving the right-aligned badge within the available width

#### Scenario: Session has ten or more tabs
- **WHEN** tab positions require multiple digits
- **THEN** the plugin right-aligns all displayed positions to the width of the largest position

#### Scenario: Tab has no agent status
- **WHEN** no tracked agent pane belongs to a tab
- **THEN** the row uses the existing position-and-name format without a status suffix

#### Scenario: Active tab has an agent status
- **WHEN** the active tab row contains a status badge
- **THEN** selected styling spans the complete fitted row including the badge
