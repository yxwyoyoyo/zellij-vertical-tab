# agent-status Specification

## Purpose

Define how supported-agent lifecycle state and native attention are published to Zellij, validated and tracked per terminal pane, synchronized across sidebar instances, cleared with pane or session lifecycle changes, and rendered as pane-aware status badges.
## Requirements
### Requirement: Agent lifecycle status publication
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

### Requirement: Native attention notification
The Codex TUI SHALL request native terminal attention for completed turns and approval requests, using BEL so Zellij can flash an active tab or retain attention for an inactive tab independently of Codex's terminal-focus detection.

#### Scenario: A Codex turn completes
- **WHEN** Codex emits `agent-turn-complete`
- **THEN** Codex emits BEL and Zellij records native bell attention for the owning tab

#### Scenario: A Codex session requests approval
- **WHEN** Codex emits `approval-requested`
- **THEN** Codex emits BEL and Zellij records native bell attention for the owning tab

#### Scenario: Codex remains terminal-focused after a Zellij tab switch
- **WHEN** either configured notification event occurs after the user moved to another Zellij tab but Codex still observes terminal focus
- **THEN** Codex emits BEL because the notification condition is `always`
- **AND** Zellij retains attention for the inactive owning tab

#### Scenario: A completed turn asks for user input
- **WHEN** Codex finishes a turn whose answer asks the user a question
- **THEN** the event uses the completed-turn notification because Codex exposes no separate structured waiting-for-answer event

### Requirement: Focus acknowledges completed agent status
The plugin SHALL treat focus of a terminal pane as acknowledgement of that pane's current completed lifecycle record and SHALL present the acknowledged record as `idle` without replacing the source lifecycle record.

#### Scenario: User focuses a completed pane
- **WHEN** a terminal pane whose current agent state is `done` becomes the focused pane of the active tab
- **THEN** the plugin records acknowledgement of that exact session and status timestamp
- **AND** the pane's rendered agent state becomes `idle`

#### Scenario: Inactive sidebar instance reports local tab focus
- **WHEN** a sidebar instance reports its containing tab as locally active but Zellij's attached-client metadata shows the user viewing another tab
- **THEN** the plugin acknowledges only completed panes in tabs viewed by attached clients
- **AND** does not acknowledge the completed pane in the locally active but unseen tab

#### Scenario: Focused pane completes without a later focus transition
- **WHEN** a `done` update is accepted while cached focus still identifies that terminal pane
- **THEN** the plugin keeps the lifecycle record unacknowledged and renders `done`
- **AND** a later focus update for another pane does not acknowledge the completed pane

#### Scenario: User returns after completion
- **WHEN** a completed pane enters the client-viewed focused-pane set after previously being absent
- **THEN** the plugin acknowledges that exact lifecycle record
- **AND** the pane's rendered agent state becomes `idle`

#### Scenario: Plugin initializes while a completed pane is focused
- **WHEN** startup or hot reload produces the first complete focus observation
- **THEN** the plugin records that observation as its focus baseline
- **AND** does not treat initialization as acknowledgement

#### Scenario: Active work is focused
- **WHEN** a focused pane's current agent state is `working` or `waiting`
- **THEN** the plugin retains and renders that state without acknowledging it

#### Scenario: New lifecycle update follows acknowledgement
- **WHEN** a pane has an acknowledged `done` record and the plugin accepts a lifecycle record with a different session ID or timestamp
- **THEN** the prior acknowledgement does not alter the new record's rendered state

#### Scenario: Acknowledged pane closes
- **WHEN** `PaneUpdate` no longer contains a terminal pane with retained acknowledgement state
- **THEN** the plugin removes that pane's acknowledgement state

### Requirement: Sidebar acknowledgement synchronization
The plugin SHALL synchronize focus acknowledgements across vertical-sidebar instances without publishing a fabricated Codex lifecycle update.

#### Scenario: Another sidebar observes the user leaving a completed pane
- **WHEN** a sidebar instance observes a changed client-viewed terminal-pane set
- **THEN** it sends that focus observation to its peer sidebar instances
- **AND** peers replace their prior focus baseline without forwarding the observation

#### Scenario: User returns through a different sidebar instance
- **WHEN** peer focus observations establish that a completed pane was absent and later newly focused
- **THEN** the receiving instance acknowledges the pane's exact current `done` record
- **AND** synchronizes the resulting acknowledgement across sidebar instances

#### Scenario: New sidebar peer is discovered
- **WHEN** a sidebar discovers a new peer while it has a complete focus baseline
- **THEN** it sends the current focus observation to that peer in addition to requesting lifecycle synchronization

#### Scenario: Sidebar acknowledges a completed record
- **WHEN** one sidebar instance newly acknowledges a `done` record because its pane is focused
- **THEN** it sends the pane ID, session ID, and acknowledged lifecycle timestamp to peer sidebar instances
- **AND** peers apply the acknowledgement without forwarding it again

#### Scenario: Peer receives acknowledgement before status
- **WHEN** a sidebar instance receives a valid acknowledgement before it receives the matching lifecycle record
- **THEN** it retains the acknowledgement reference
- **AND** presents a later matching `done` record as `idle`

#### Scenario: New sidebar joins after acknowledgement
- **WHEN** a new sidebar instance requests a synchronization snapshot after a completed record has been acknowledged
- **THEN** the snapshot includes both the lifecycle record and its acknowledgement reference
- **AND** the new instance presents the record as `idle`

#### Scenario: Malformed acknowledgement arrives
- **WHEN** an internal acknowledgement message has an invalid pane ID, empty session ID, unsupported version, or invalid timestamp
- **THEN** the receiving sidebar ignores it without changing rendered state

#### Scenario: Malformed focus observation arrives
- **WHEN** an internal focus message has an unsupported version, invalid pane ID, or duplicate pane ID
- **THEN** the receiving sidebar ignores it without changing focus or rendered state

### Requirement: Durable lifecycle journal
The Codex lifecycle and notification bridges SHALL durably record each valid pane lifecycle payload before best-effort publication so the newest status, including a `clear` tombstone, remains recoverable while no sidebar plugin runtime is loaded.

#### Scenario: Lifecycle event occurs while attached
- **WHEN** a bridge creates a valid lifecycle payload and identifies its Zellij server process
- **THEN** it atomically records that payload under the server and terminal pane before publishing the normal pipe message

#### Scenario: Lifecycle event occurs while detached
- **WHEN** a valid lifecycle or exit-watcher event occurs while Zellij has no attached client and its undirected pipe is not delivered
- **THEN** the host journal still retains the event for later recovery

#### Scenario: Old session clears a reused pane
- **WHEN** a `clear` payload names a different Codex session than the newer journal record for the same server and pane
- **THEN** the journal ignores the clear and retains the newer session

#### Scenario: Older payload races a newer writer
- **WHEN** concurrent bridge processes attempt to record payloads for the same server and pane
- **THEN** per-pane serialization and timestamp validation retain the newest applicable record without producing partial JSON

#### Scenario: Zellij server identity is unavailable
- **WHEN** a bridge cannot identify a positive Zellij server process ID
- **THEN** it skips durable recording and continues its existing best-effort publication behavior

### Requirement: Sidebar state recovery
The plugin SHALL recover validated lifecycle records and exact focus acknowledgements after its runtime restarts, then reconcile lifecycle records emitted while detached without treating recovery as a focus transition.

#### Scenario: Client detaches and reattaches
- **WHEN** Zellij recreates a sidebar runtime for the same live server after detach or session switch
- **THEN** the sidebar restores its cached lifecycle and acknowledgement snapshot before normal peer convergence

#### Scenario: Completed result was previously acknowledged
- **WHEN** a cached acknowledgement exactly matches a restored `done` record
- **THEN** the sidebar continues to present that record as `idle`

#### Scenario: Completion remained unseen before detach
- **WHEN** a restored `done` record has no exact acknowledgement
- **THEN** the sidebar continues to present it as `done`
- **AND** startup does not fabricate a focus acknowledgement

#### Scenario: Codex exits while detached
- **WHEN** plugin cache contains `done` but the host recovery snapshot contains a newer matching-session `clear`
- **THEN** the sidebar retains the clear tombstone and renders no status

#### Scenario: Work changes while detached
- **WHEN** the host recovery snapshot contains a lifecycle record newer than plugin cache
- **THEN** the sidebar presents the newer recovered state

#### Scenario: Live update races recovery
- **WHEN** a live pipe record and recovery snapshot target the same pane
- **THEN** the existing timestamp and session rules determine the current record regardless of arrival order

#### Scenario: Cached pane no longer exists
- **WHEN** `PaneUpdate` omits a terminal pane named by recovered state
- **THEN** the plugin removes that pane's recovered lifecycle and acknowledgement state

### Requirement: Best-effort recovery boundary
The plugin SHALL validate and bound all persistent recovery input and SHALL continue operating with in-memory state when durable recovery is unavailable.

#### Scenario: Recovery helper succeeds
- **WHEN** the fixed helper command returns successful bounded UTF-8 output with the expected context and a valid version-1 snapshot
- **THEN** the plugin merges it through the normal snapshot validation path

#### Scenario: Recovery permission is denied
- **WHEN** Zellij denies permission to run the recovery helper
- **THEN** the plugin continues rendering cached, live, and peer-synchronized state without crashing or repeatedly prompting

#### Scenario: Helper or journal is unavailable
- **WHEN** helper execution fails, returns nonzero, exceeds the output limit, or has malformed output
- **THEN** the plugin ignores that result without replacing valid current state

#### Scenario: Plugin cache entry is invalid
- **WHEN** a matching cache file is malformed, oversized, unreadable, or has an invalid filename
- **THEN** the plugin skips that entry and continues scanning other entries

#### Scenario: Another Zellij server reuses pane IDs
- **WHEN** another live Zellij server has the same terminal or plugin pane IDs
- **THEN** server-PID namespacing prevents its persistent state from being loaded

### Requirement: Bounded lifecycle integration overhead
The agent-status integration SHALL bound long-lived helper count, cross-instance message amplification, and persistent recovery discovery independently of repeated session-start events and historical server count.

#### Scenario: Session start repeats for one Codex process
- **WHEN** startup, resume, clear, or compact events invoke the lifecycle bridge more than once for the same live Codex PID
- **THEN** at most one detached exit watcher remains active for that process
- **AND** process exit still publishes one applicable clear record

#### Scenario: Platform supports process-exit events
- **WHEN** the watcher can register an event-driven process-exit notification
- **THEN** it waits without periodic polling

#### Scenario: Lifecycle status is broadcast to existing sidebars
- **WHEN** an untargeted named Zellij pipe delivers one lifecycle update to every listening sidebar instance
- **THEN** each instance applies the update without forwarding another copy to its peers

#### Scenario: Focus changes across many sidebar instances
- **WHEN** multiple sidebar instances observe the same focus transition
- **THEN** nonleaders report the transition only to one elected live instance
- **AND** only that leader fans focus and acknowledgement messages out to peers
- **AND** another live instance assumes leadership after the leader closes

#### Scenario: New sidebar joins after an earlier lifecycle event
- **WHEN** a sidebar instance starts after the external lifecycle broadcast
- **THEN** peer snapshot synchronization restores the current status and acknowledgement state

#### Scenario: Current server restores plugin cache
- **WHEN** plugin cache contains snapshots from current and historical Zellij servers
- **THEN** normal restore scans the current server shard rather than every historical snapshot

#### Scenario: Legacy flat cache exists during upgrade
- **WHEN** the current server has compatible flat cache snapshots and no server shard
- **THEN** the first updated instance merges and migrates that state without losing lifecycle records or acknowledgements

#### Scenario: Dead host journals accumulate
- **WHEN** session-start maintenance finds a numeric journal directory whose Zellij PID no longer exists
- **THEN** it removes that dead server directory
- **AND** retains directories for live or permission-inaccessible PIDs

#### Scenario: Session start discovers process ownership
- **WHEN** the bridge needs both Codex and Zellij ancestor PIDs
- **THEN** it resolves both identities in one bounded ancestor traversal

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

### Requirement: Common agent adapter interface
The project SHALL provide a dependency-free common bridge interface that accepts normalized lifecycle updates from supported-agent adapters and owns their conversion into the existing versioned Zellij status protocol.

#### Scenario: Adapter emits a normalized update
- **WHEN** a supported-agent adapter supplies a non-empty session ID, canonical state, canonical lifecycle event, and optional non-empty turn identity
- **THEN** the common bridge obtains the terminal pane ID from Zellij context and assigns the publication timestamp
- **AND** it builds the same validated version-1 payload regardless of the native agent

#### Scenario: Common bridge dispatches an update
- **WHEN** a normalized update is valid and the bridge can identify its Zellij server
- **THEN** the common bridge journals the payload before publishing it on `vertical-tab-agent-status`
- **AND** storage or publication failure remains best-effort and cannot alter the agent's lifecycle decision

#### Scenario: Adapter runs outside Zellij
- **WHEN** a supported-agent adapter runs without `ZELLIJ_PANE_ID`
- **THEN** the common bridge exits successfully without constructing, storing, or publishing a status payload

#### Scenario: Plugin requests detached-state recovery
- **WHEN** an installed supported-agent entrypoint receives a valid snapshot request
- **THEN** it delegates to the common bridge and returns the shared server-scoped journal snapshot

#### Scenario: Agent requires native extensions
- **WHEN** an agent needs behavior outside normalized lifecycle publication, such as process-exit watching, notifier forwarding, or a terminal-sequence hook response
- **THEN** that behavior remains in the agent-specific adapter
- **AND** any resulting lifecycle update still passes through the common bridge interface

#### Scenario: User installs an agent adapter
- **WHEN** the user installs or updates a supported-agent bridge
- **THEN** the common bridge runtime and durable store are copied beside that agent's entrypoint
- **AND** no package manager, virtual environment, or globally importable Python package is required
