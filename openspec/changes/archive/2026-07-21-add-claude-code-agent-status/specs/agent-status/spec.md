## MODIFIED Requirements

### Requirement: Codex lifecycle status publication
The project SHALL provide globally installable Codex and Claude Code lifecycle bridges that translate supported events into the same versioned status protocol for the Zellij terminal pane that owns each agent session, regardless of the session's working directory and without an agent-name prefix in the rendered badge.

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

#### Scenario: Claude Code session starts inside Zellij
- **WHEN** a Claude Code `SessionStart` hook runs with `ZELLIJ_PANE_ID` available
- **THEN** the Claude bridge publishes an `idle` status containing the terminal pane ID, Claude session ID, and publication timestamp

#### Scenario: Claude Code begins or continues work
- **WHEN** a `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, or `PermissionDenied` hook runs inside Zellij
- **THEN** the Claude bridge publishes `working` for that pane and session
- **AND** it includes the normalized lifecycle event and any non-empty Claude prompt ID as the protocol turn identity

#### Scenario: Claude Code requests permission
- **WHEN** a Claude Code `PermissionRequest` hook runs because a permission dialog is about to be shown
- **THEN** the bridge publishes `waiting` for that pane and session

#### Scenario: Claude Code finishes responding
- **WHEN** a Claude Code `Stop` hook runs
- **THEN** the bridge publishes `done` for that pane and session
- **AND** the existing focus-acknowledgement behavior applies without distinguishing the agent implementation

#### Scenario: Claude Code requests user attention
- **WHEN** a Claude Code `PermissionRequest` hook runs inside Zellij
- **THEN** the bridge returns a supported terminal-sequence response containing one BEL
- **AND** Zellij can retain native tab attention while the pane status remains `waiting`

#### Scenario: Claude Code presents a final answer
- **WHEN** a Claude Code `Stop` hook runs inside Zellij without `stop_hook_active` set to true
- **THEN** the bridge returns a supported terminal-sequence response containing one BEL
- **AND** Zellij can retain native tab attention while the pane status remains `done`

#### Scenario: Claude Code continues through a stop hook
- **WHEN** a Claude Code `Stop` hook runs with `stop_hook_active` set to true
- **THEN** the bridge emits no terminal notification
- **AND** it does not falsely announce that the continuing response is final

#### Scenario: Claude Code session ends or switches
- **WHEN** a Claude Code `SessionEnd` hook runs for a current session
- **THEN** the bridge journals and publishes a matching-session `clear` tombstone
- **AND** a clear from an older session cannot remove a newer session in the reused pane

#### Scenario: Codex and Claude Code run concurrently
- **WHEN** Codex and Claude Code sessions run in separate terminal panes or tabs
- **THEN** each lifecycle record remains associated with its exact terminal pane
- **AND** the sidebar renders the same agent-neutral badge vocabulary for both agents

#### Scenario: Agent bridge runs outside Zellij
- **WHEN** either lifecycle bridge runs without `ZELLIJ_PANE_ID`
- **THEN** it exits successfully without publishing or changing stored status

## ADDED Requirements

### Requirement: Durable agent-status recovery
The plugin SHALL recover validated lifecycle records emitted by either supported agent while sidebar runtimes are unloaded, using the shared host journal and an installed supported-agent snapshot helper.

#### Scenario: Only the Claude Code bridge is installed
- **WHEN** the plugin requests a host-journal snapshot and the Codex helper is unavailable but the Claude helper is executable
- **THEN** the fixed recovery command invokes the Claude helper for the current Zellij server PID
- **AND** the returned snapshot passes through the existing bounded validation and ordering rules

#### Scenario: Claude lifecycle changes while detached
- **WHEN** Claude Code emits working, waiting, done, or clear while no sidebar runtime can receive the live pipe
- **THEN** the bridge first journals that record under the owning Zellij server and terminal pane
- **AND** reattachment reconciles the newer journal record over stale plugin cache

### Requirement: Global Claude Code hook installation
The project SHALL provide a user-level Claude Code hook template and installation instructions that preserve unrelated user configuration.

#### Scenario: User has existing Claude settings
- **WHEN** the user installs Claude Code status support and `~/.claude/settings.json` already contains environment, theme, permissions, or hook configuration
- **THEN** the documented workflow merges the new hook groups and handlers without replacing unrelated settings

#### Scenario: Hook bridge failure
- **WHEN** hook input is malformed, Zellij context is absent, storage is unavailable, or pipe publication fails
- **THEN** the bridge exits successfully and does not block or alter Claude Code's lifecycle decision

#### Scenario: Lifecycle event does not require attention
- **WHEN** the Claude bridge handles an event other than `PermissionRequest` or a final `Stop`
- **THEN** it emits no terminal sequence
