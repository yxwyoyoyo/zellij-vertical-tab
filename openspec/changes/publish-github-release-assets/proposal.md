## Why

Users currently have to install the Rust toolchain and build the plugin locally even when they only want to run a published version. A versioned GitHub Release should provide the checked release WASM directly, together with an integrity checksum.

## What Changes

- Add a maintainer-run publishing command that validates a semantic version tag, repository state, package version, GitHub authentication, and release uniqueness.
- Build through the existing checked release workflow and publish the resulting WASM plus a SHA-256 checksum as GitHub Release assets.
- Document versioned and latest-release download URLs and the Zellij compatibility constraint.
- Keep publishing local and explicit; do not add a GitHub Actions workflow.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: Extend the release artifact contract to cover downloadable GitHub Release assets, checksums, and guarded maintainer publishing.

## Impact

The change affects `mise.toml`, a new script under `scripts/`, release documentation in `README.md` and `DEVELOPMENT.md`, the generated OpenWiki, and the vertical-tab sidebar release specification. It uses the existing `gh` CLI and does not introduce a runtime dependency or hosted CI workflow.
