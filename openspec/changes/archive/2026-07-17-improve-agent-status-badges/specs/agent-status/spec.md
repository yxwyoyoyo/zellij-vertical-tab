## ADDED Requirements

### Requirement: Aligned theme-colored status badges
The plugin SHALL render each agent state with a single-cell icon provided by Nerd Font Mono and SHALL apply a distinct Zellij theme style to the complete aggregate badge without changing the selected row's background styling.

#### Scenario: Idle status is rendered
- **WHEN** a tab's dominant agent state is `idle`
- **THEN** its badge uses the native Nerd Font circle-outline icon ``
- **AND** the complete badge uses Zellij's dim text style

#### Scenario: Working status is rendered
- **WHEN** a tab's dominant agent state is `working`
- **THEN** its badge uses the native Nerd Font filled-circle icon ``
- **AND** the complete badge uses Zellij text emphasis level 1

#### Scenario: Waiting status is rendered
- **WHEN** a tab's dominant agent state is `waiting`
- **THEN** its badge uses the native Nerd Font clock icon ``
- **AND** the complete badge uses Zellij text emphasis level 0

#### Scenario: Done status is rendered
- **WHEN** a tab's dominant agent state is `done`
- **THEN** its badge uses the native Nerd Font check-circle icon ``
- **AND** the complete badge uses Zellij's semantic success style

#### Scenario: Multiple panes share a badge
- **WHEN** a badge includes a pane count after its dominant-state icon
- **THEN** the icon and count receive the same state-specific text style

#### Scenario: Active tab has a colored badge
- **WHEN** a colored badge belongs to the active tab row
- **THEN** the badge retains its state-specific foreground style
- **AND** selected styling continues across the complete row background

#### Scenario: Icons are measured in the configured font
- **WHEN** the documented Nerd Font Mono requirement is satisfied
- **THEN** every state icon occupies one terminal cell and uses the font's shared native icon metrics instead of a fallback font
