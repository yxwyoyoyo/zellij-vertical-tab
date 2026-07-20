## ADDED Requirements

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
