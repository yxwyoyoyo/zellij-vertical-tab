## ADDED Requirements

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
