## MODIFIED Requirements

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

#### Scenario: Stale pane records accumulate within a live server directory
- **WHEN** session-start maintenance runs for a live Zellij server directory
- **THEN** it removes pane records whose state is `"clear"` (explicitly expired via SessionEnd)
- **AND** it removes pane records older than a 6-hour grace period from a different session than the current starting session
- **AND** it preserves records matching the current session and those recently updated from other concurrent sessions
- **AND** each removal acquires the per-pane advisory lock before unlinking the record and lock files

#### Scenario: Session start discovers process ownership
- **WHEN** the bridge needs both Codex and Zellij ancestor PIDs
- **THEN** it resolves both identities in one bounded ancestor traversal
