## MODIFIED Requirements

### Requirement: Multi-pane tab aggregation
The plugin SHALL place a prefix-free agent status badge on the row that most precisely represents its owning terminal pane while keeping tabs with at most one terminal pane compact.

#### Scenario: Tab has one terminal pane with status
- **WHEN** a tab contains exactly one terminal pane and that pane has a renderable state
- **THEN** the tab row displays that pane's state glyph without a pane count
- **AND** no pane child row is rendered

#### Scenario: Tab has multiple terminal panes with statuses
- **WHEN** a tab contains more than one terminal pane
- **THEN** each pane child row displays only that pane's renderable state glyph when one exists
- **AND** the parent tab row displays no aggregate badge or pane count

#### Scenario: Only some panes have status
- **WHEN** a multi-pane tab contains terminal panes without tracked agent state
- **THEN** those pane rows render without a badge while tracked pane rows retain their own badges

#### Scenario: Tab has no tracked Codex pane
- **WHEN** no tracked terminal pane with a renderable state belongs to a tab
- **THEN** the tab and pane rows have no agent status badge

### Requirement: Aligned theme-colored status badges
The plugin SHALL render each agent state with a single-cell icon provided by Nerd Font Mono and SHALL apply a distinct Zellij theme style to the complete badge without changing selected row background styling.

#### Scenario: Idle status is rendered
- **WHEN** a tab or pane row represents an `idle` agent state
- **THEN** its badge uses the native Nerd Font circle-outline icon ``
- **AND** the complete badge uses Zellij's dim text style

#### Scenario: Working status is rendered
- **WHEN** a tab or pane row represents a `working` agent state
- **THEN** its badge uses the native Nerd Font filled-circle icon ``
- **AND** the complete badge uses Zellij text emphasis level 1

#### Scenario: Waiting status is rendered
- **WHEN** a tab or pane row represents a `waiting` agent state
- **THEN** its badge uses the native Nerd Font clock icon ``
- **AND** the complete badge uses Zellij text emphasis level 0

#### Scenario: Done status is rendered
- **WHEN** a tab or pane row represents a `done` agent state
- **THEN** its badge uses the native Nerd Font check-circle icon ``
- **AND** the complete badge uses Zellij's semantic success style

#### Scenario: Selected row has a colored badge
- **WHEN** a colored badge belongs to an active tab row or focused pane child row
- **THEN** the badge retains its state-specific foreground style
- **AND** selected styling continues across the complete row background

#### Scenario: Icons are measured in the configured font
- **WHEN** the documented Nerd Font Mono requirement is satisfied
- **THEN** every state icon occupies one terminal cell and uses the font's shared native icon metrics instead of a fallback font
