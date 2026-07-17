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

### Requirement: Overflow navigation
The plugin SHALL support bounded vertical navigation when the tab count exceeds the available sidebar rows.

#### Scenario: Tabs exist above the visible window
- **WHEN** the first displayed tab is not the first session tab
- **THEN** the first row uses an upward overflow indicator

#### Scenario: Tabs exist below the visible window
- **WHEN** the last displayed tab is not the last session tab
- **THEN** the last row uses a downward overflow indicator

#### Scenario: User scrolls the sidebar
- **WHEN** the user scrolls up or down over the sidebar
- **THEN** the visible window moves by one row without passing its valid bounds

### Requirement: Mouse tab switching
The plugin SHALL switch to the tab represented by a valid left-clicked sidebar row.

#### Scenario: User clicks a tab row
- **WHEN** the user left-clicks a row that maps to a visible tab
- **THEN** the plugin asks Zellij to switch to that tab using its one-based position

#### Scenario: User clicks outside the rendered rows
- **WHEN** the user left-clicks a row that does not map to a tab
- **THEN** the plugin performs no tab-switching action

### Requirement: Session-wide layout integration
The development layout SHALL place the sidebar in every tab, replace the built-in horizontal tab bar, and retain the built-in status bar.

#### Scenario: Session starts from the development layout
- **WHEN** Zellij loads `zellij.kdl`
- **THEN** every tab receives a borderless 24-column vertical sidebar
- **AND** normal tab children occupy the sibling pane
- **AND** the built-in status bar occupies the bottom row

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
