## Context

The repository already pins its toolchain with mise and produces an optimized `wasm32-wasip1` artifact through `mise run release`. Installation currently assumes a local checkout and Rust toolchain. The public GitHub repository has no releases, tags, or product CI, and a previous generated OpenWiki workflow was intentionally removed.

## Goals / Non-Goals

**Goals:**

- Make a checked, versioned WASM directly downloadable from GitHub Releases.
- Make the maintainer publishing operation reproducible and resistant to common versioning and repository-state mistakes.
- Publish an integrity checksum and document compatibility and stable download URLs.

**Non-Goals:**

- Add GitHub Actions or scheduled automation.
- Build artifacts for multiple Zellij ABI versions in one release.
- Auto-increment package versions or infer the release version.
- Publish from a dirty tree or a branch other than the remote default branch.

## Decisions

### Publish through a local mise task

`mise run publish -- vX.Y.Z` invokes a checked-in shell script. The script reuses `mise run release`, so the same format, tests, Clippy, WASM build, OpenSpec validation, and diff checks gate both local installation and public distribution.

This is preferred over GitHub Actions because the project already has a reproducible local toolchain and does not otherwise need hosted CI. Manual browser uploads were rejected because they can bypass the release gate or select a stale artifact.

### Validate identity before building or publishing

The command requires a clean tracked and untracked worktree, the remote default branch, a package version equal to the tag without its `v` prefix, authenticated `gh`, an exact match between local `HEAD` and the remote default branch, and absence of an existing tag or release. These checks make the tag, source commit, and compiled asset unambiguous.

### Let `gh release create` stage and publish the release

The script builds into the normal Cargo target directory, copies the WASM into a temporary staging directory, creates a SHA-256 checksum there, and passes both assets to `gh release create`. GitHub CLI internally creates a draft, uploads assets, and publishes only after the uploads succeed. Generated notes are prefixed with the supported Zellij version.

The stable asset name is `zellij_vertical_tab.wasm`, enabling GitHub's `/releases/latest/download/...` URL.

### Keep the source version authoritative

The release tag must exactly match the `[package]` version in `Cargo.toml`. The script does not edit source files during publishing; version changes remain reviewable repository changes.

## Risks / Trade-offs

- **A maintainer's local environment produces the public binary** → `mise run release` pins Rust, validates the full project, and builds from the exact remote-default commit.
- **A network failure can interrupt publishing** → GitHub CLI uploads through a draft before publication; a remaining draft can be inspected or removed before retrying.
- **The artifact only supports one Zellij plugin ABI** → release notes and installation docs explicitly state the matching Zellij version.
- **A checksum does not provide signed provenance** → SHA-256 provides corruption detection now; signing or attestations can be introduced separately.
- **Strict clean-tree checks make the publishing script hard to test before commit** → local validation tests its syntax and failure paths; the successful path runs only after the change is committed and pushed.
