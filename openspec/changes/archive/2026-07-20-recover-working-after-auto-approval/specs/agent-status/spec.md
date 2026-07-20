## MODIFIED Requirements

### Requirement: Codex lifecycle status publication
The project SHALL provide globally installable Codex lifecycle and completion-notifier bridges that translate supported events into versioned status messages for the Zellij terminal pane that owns the Codex session, regardless of the session's working directory.

#### Scenario: Codex starts in another project
- **WHEN** Codex runs inside Zellij from a directory outside the plugin repository
- **THEN** the globally installed hook publishes status for that terminal pane without requiring a project-local hook copy

#### Scenario: Codex session starts inside Zellij
- **WHEN** a `SessionStart` hook runs with `ZELLIJ_PANE_ID` available
- **THEN** the hook bridge publishes an `idle` status containing the terminal pane ID, Codex session ID, and publication timestamp

#### Scenario: Codex begins or continues work
- **WHEN** a `UserPromptSubmit`, `PreToolUse`, or `PostToolUse` hook runs inside Zellij
- **THEN** the hook bridge publishes a `working` status for that pane and session
- **AND** it includes the normalized lifecycle event and any non-empty Codex turn ID

#### Scenario: Codex requests permission
- **WHEN** a `PermissionRequest` hook runs inside Zellij and its matching turn context identifies a user reviewer
- **THEN** the hook bridge publishes a `waiting` status for that pane and session

#### Scenario: Automatic review handles permission
- **WHEN** a `PermissionRequest` hook runs and its matching turn context identifies `auto_review`
- **THEN** the hook bridge publishes `working` because no user response is required

#### Scenario: Reviewer context is unavailable
- **WHEN** a `PermissionRequest` hook cannot read a matching supported reviewer identity
- **THEN** the hook bridge conservatively publishes `waiting`

#### Scenario: Manually approved tool finishes
- **WHEN** the user approves a request and Codex later emits `PostToolUse`
- **THEN** the bridge returns the pane from `waiting` to `working`

#### Scenario: Codex finishes a turn
- **WHEN** a `Stop` hook or `agent-turn-complete` notification runs inside Zellij
- **THEN** the applicable bridge publishes a `done` status for that pane and session

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
- **WHEN** the designated pipe carries a version 1 payload with a terminal pane ID, non-empty session ID, supported state, timestamp, and optional supported lifecycle event and turn ID
- **THEN** the plugin applies the message to that terminal pane

#### Scenario: Legacy version-1 message arrives
- **WHEN** a valid version 1 live message or persistent snapshot omits lifecycle event or turn identity
- **THEN** the plugin accepts it using conservative legacy ordering

#### Scenario: Unsupported lifecycle metadata arrives
- **WHEN** a message contains an unknown lifecycle event or an empty turn ID
- **THEN** the plugin ignores it without requesting a render

#### Scenario: Unsupported or malformed message arrives
- **WHEN** a message uses another pipe name, unsupported version, invalid pane ID, empty session ID, unknown state, or malformed JSON
- **THEN** the plugin ignores it without requesting a render

### Requirement: Per-pane current session tracking
The plugin SHALL retain at most one current agent status record per terminal pane, SHALL prevent older messages from overwriting newer state, and SHALL prevent delayed events from reopening a completed turn.

#### Scenario: Current session changes state
- **WHEN** a message for the same pane and session has a timestamp equal to or newer than the stored record
- **THEN** the plugin replaces the stored state and timestamp unless terminal-done ordering rejects the update

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

#### Scenario: Approval completes and agent resumes
- **WHEN** a newer `PostToolUse` working record follows a waiting record for the same pane, session, and turn
- **THEN** the plugin replaces waiting with working

#### Scenario: Delayed event follows completion in the same turn
- **WHEN** a `PreToolUse`, `PermissionRequest`, or `PostToolUse` record arrives after `done` for the same pane, session, and turn
- **THEN** the plugin retains done

#### Scenario: Completion lacks turn identity
- **WHEN** a done record or later tool event lacks turn identity
- **THEN** only a later `UserPromptSubmit` may reopen the completed session as working

#### Scenario: New prompt begins another turn
- **WHEN** a newer `UserPromptSubmit` follows done for the same pane and session
- **THEN** the plugin accepts working for the new turn and removes any superseded focus acknowledgement

#### Scenario: Persistent sources merge lifecycle records
- **WHEN** host journal, plugin cache, peer snapshot, or live pipe records target the same pane
- **THEN** every merge path applies the same timestamp, session, turn, and terminal-done rules
