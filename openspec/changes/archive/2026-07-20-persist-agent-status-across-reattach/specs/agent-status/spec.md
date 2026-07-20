## ADDED Requirements

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
