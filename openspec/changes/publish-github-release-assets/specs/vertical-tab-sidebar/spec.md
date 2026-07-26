## ADDED Requirements

### Requirement: Downloadable release artifact
The project SHALL publish each supported version as a GitHub Release containing the checked release WASM and a SHA-256 checksum under stable asset names.

#### Scenario: User downloads the latest release
- **WHEN** a user requests the latest `zellij_vertical_tab.wasm` release asset
- **THEN** GitHub serves the optimized `wasm32-wasip1` artifact without requiring a local Rust build
- **AND** the same release provides `zellij_vertical_tab.wasm.sha256`
- **AND** the release notes identify the compatible Zellij version

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
