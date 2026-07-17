# agent-status Specification

## Purpose

Define how Codex lifecycle state is published to Zellij, validated and tracked per terminal pane, synchronized across sidebar instances, cleared with pane or session lifecycle changes, and rendered as pane-aware status badges.

## Requirements
### Requirement: Codex lifecycle status publication
The project SHALL provide globally installable Codex lifecycle and completion-notifier bridges that translate supported events into versioned status messages for the Zellij terminal pane that owns the Codex session, regardless of the session's working directory.

#### Scenario: Codex starts in another project
- **WHEN** Codex runs inside Zellij from a directory outside the plugin repository
- **THEN** the globally installed hook publishes status for that terminal pane without requiring a project-local hook copy

#### Scenario: Codex session starts inside Zellij
- **WHEN** a `SessionStart` hook runs with `ZELLIJ_PANE_ID` available
- **THEN** the hook bridge publishes an `idle` status containing the terminal pane ID, Codex session ID, and publication timestamp

#### Scenario: Codex begins or continues work
- **WHEN** a `UserPromptSubmit` or `PreToolUse` hook runs inside Zellij
- **THEN** the hook bridge publishes a `working` status for that pane and session

#### Scenario: Codex requests permission
- **WHEN** a `PermissionRequest` hook runs inside Zellij
- **THEN** the hook bridge publishes a `waiting` status for that pane and session

#### Scenario: Codex finishes a turn
- **WHEN** a `Stop` hook runs inside Zellij
- **THEN** the hook bridge publishes a `done` status for that pane and session

#### Scenario: A completion workflow omits Stop
- **WHEN** Codex emits an external `agent-turn-complete` notification for a turn, including special workflows such as code review
- **THEN** the notification bridge publishes `done` for that pane and session

#### Scenario: An existing external notifier is configured
- **WHEN** the notification bridge handles an `agent-turn-complete` notification
- **THEN** it publishes agent status and forwards the unmodified payload to the previously configured notifier

#### Scenario: Another notification type is received
- **WHEN** the notification bridge handles a notification other than `agent-turn-complete`
- **THEN** it forwards the unmodified payload without publishing agent status

#### Scenario: Codex exits while its terminal pane remains open
- **WHEN** the Codex process associated with a tracked session exits
- **THEN** the hook bridge publishes a `clear` status for that pane and session

#### Scenario: A late tool event follows completion
- **WHEN** a `PostToolUse` hook completes after the turn's `Stop` hook
- **THEN** it does not publish a new `working` status or overwrite `done`

#### Scenario: Status bridge runs outside Zellij
- **WHEN** either status bridge has no Zellij pane ID or cannot contact the current Zellij session
- **THEN** it exits successfully without blocking Codex, and the notification bridge still invokes any configured forwarded notifier

### Requirement: Validated status transport
The plugin SHALL accept agent status only from the designated pipe name and SHALL validate the complete versioned payload before changing state.

#### Scenario: Supported message arrives
- **WHEN** the designated pipe carries a version 1 payload with a terminal pane ID, non-empty session ID, supported state, and timestamp
- **THEN** the plugin applies the message to that terminal pane

#### Scenario: Unsupported or malformed message arrives
- **WHEN** a message uses another pipe name, unsupported version, invalid pane ID, empty session ID, unknown state, or malformed JSON
- **THEN** the plugin ignores it without requesting a render

### Requirement: Per-pane current session tracking
The plugin SHALL retain at most one current agent status record per terminal pane and SHALL prevent older messages from overwriting newer state.

#### Scenario: Current session changes state
- **WHEN** a message for the same pane and session has a timestamp equal to or newer than the stored record
- **THEN** the plugin replaces the stored state and timestamp

#### Scenario: New session reuses a pane
- **WHEN** a newer message with a different session ID arrives for an already tracked pane
- **THEN** the plugin replaces the previous session record rather than increasing the pane count

#### Scenario: Older message arrives late
- **WHEN** a message timestamp is older than the stored record for that pane
- **THEN** the plugin ignores the message

#### Scenario: Explicit clear message arrives
- **WHEN** a valid `clear` message arrives for a pane, matches the stored session ID, and is not older than its stored record
- **THEN** the plugin removes the pane's rendered status and retains a non-rendered timestamp tombstone

#### Scenario: Delayed status follows clear
- **WHEN** a status update or peer snapshot older than a retained clear tombstone arrives for that pane
- **THEN** the plugin rejects it instead of resurrecting the cleared status

#### Scenario: Old session exits after pane reuse
- **WHEN** a `clear` message names a different session from the current record for that pane
- **THEN** the plugin retains the current session's status

### Requirement: Pane lifecycle cleanup
The plugin SHALL associate terminal pane statuses with their owning tabs from Zellij pane state and SHALL remove records for terminal panes that no longer exist.

#### Scenario: Pane manifest associates a tracked pane
- **WHEN** `PaneUpdate` reports a tracked terminal pane under a tab position
- **THEN** the plugin includes that pane when aggregating status for the tab

#### Scenario: Tracked pane closes
- **WHEN** a later `PaneUpdate` no longer contains a tracked terminal pane
- **THEN** the plugin removes that pane's status record

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

### Requirement: Sidebar instance synchronization
The plugin SHALL synchronize agent status across all vertical-sidebar plugin instances in the Zellij session so every tab displays the same session-wide status view.

#### Scenario: One sidebar receives an external status
- **WHEN** a CLI pipe delivers a valid status to any running sidebar instance
- **THEN** that instance forwards the update to the other vertical-sidebar instances without creating a forwarding loop

#### Scenario: A new tab creates a sidebar instance
- **WHEN** a new vertical-sidebar instance discovers an existing peer
- **THEN** it requests and merges a timestamp-validated snapshot of the peer's current agent records

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
