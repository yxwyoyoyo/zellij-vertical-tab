## Why

Users currently have to install the Rust toolchain and build the plugin locally even when they only want to run a published version. The README's agent-status setup also assumes a repository checkout, so downloading the WASM alone does not provide an obvious path to install the matching Codex and Claude Code hooks. A versioned GitHub Release should provide both runtime pieces directly with integrity checksums.

## What Changes

- Add a maintainer-run publishing command that validates a semantic version tag, repository state, package version, GitHub authentication, and release uniqueness.
- Build through the existing checked release workflow and publish the resulting WASM, a version-matched agent-hook bundle, and SHA-256 checksums as GitHub Release assets.
- Document versioned and latest-release download URLs, the Zellij compatibility constraint, and concise Codex and Claude Code hook installation.
- Keep publishing local and explicit; do not add a GitHub Actions workflow.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: Extend the release artifact contract to cover downloadable WASM and agent-hook assets, checksums, installation guidance, and guarded maintainer publishing.

## Impact

The change affects `mise.toml`, a new script under `scripts/`, release documentation in `README.md` and `DEVELOPMENT.md`, the generated OpenWiki, the hook adapters under `hooks/`, and the vertical-tab sidebar release specification. It uses the existing `gh` CLI and archive tools and does not introduce a runtime dependency or hosted CI workflow.
