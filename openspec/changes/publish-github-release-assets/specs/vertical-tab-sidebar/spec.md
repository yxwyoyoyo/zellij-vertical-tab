## ADDED Requirements

### Requirement: Downloadable release artifacts
The project SHALL publish each supported version as a GitHub Release containing the checked release WASM, a version-matched Codex and Claude Code hook bundle, and SHA-256 checksums under stable asset names.

#### Scenario: User downloads the latest release
- **WHEN** a user requests the latest `zellij_vertical_tab.wasm` release asset
- **THEN** GitHub serves the optimized `wasm32-wasip1` artifact without requiring a local Rust build
- **AND** the same release provides `zellij_vertical_tab.wasm.sha256`
- **AND** the release notes identify the compatible Zellij version

#### Scenario: User downloads agent integrations
- **WHEN** a user requests the latest `agent-hooks.tar.gz` release asset
- **THEN** GitHub serves the common bridge, Codex adapter and template, and Claude Code adapter and template without requiring a repository checkout
- **AND** the same release provides `agent-hooks.tar.gz.sha256`
- **AND** the hook bundle comes from the same source version as the release WASM

#### Scenario: User installs an agent integration
- **WHEN** a user follows the README setup for Codex or Claude Code
- **THEN** the documented commands install the selected adapter and common runtime from the release bundle
- **AND** fresh configuration can use the shipped template
- **AND** existing configuration is merged rather than overwritten
- **AND** Codex completion notification configuration is documented

#### Scenario: Maintainer publishes a version
- **WHEN** a maintainer runs the documented publish command with a valid `vMAJOR.MINOR.PATCH` tag
- **THEN** the complete local release gate runs before any release is created
- **AND** the tag version must equal the package version
- **AND** the source must be a clean checkout of the remote default branch
- **AND** an existing tag or GitHub Release with that version is not replaced

#### Scenario: Release validation fails
- **WHEN** any publishing precondition or release build fails
- **THEN** the command exits without publishing a GitHub Release

### Requirement: Maintainer-run release publishing
The project SHALL keep GitHub Release publishing as an explicit local maintainer operation without requiring a hosted GitHub Actions workflow.

#### Scenario: Maintainer prepares a public release
- **WHEN** the maintainer follows the release workflow
- **THEN** a checked-in mise task invokes the guarded local publishing command
- **AND** GitHub authentication is provided by the maintainer's `gh` session
