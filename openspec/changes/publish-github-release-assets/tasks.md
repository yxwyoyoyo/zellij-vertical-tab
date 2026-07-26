## 1. Release Automation

- [x] 1.1 Add a guarded local script that validates the tag, package version, Git state, remote default branch, GitHub authentication, and release uniqueness
- [x] 1.2 Stage the checked release WASM and SHA-256 checksum and publish them with compatibility and generated release notes
- [x] 1.3 Expose the publishing script through a mise task and include its static validation in the project gate

## 2. Documentation

- [x] 2.1 Document direct release downloads and the supported Zellij version in the README
- [x] 2.2 Document the maintainer publishing and recovery workflow in DEVELOPMENT
- [x] 2.3 Regenerate OpenWiki and verify that it describes local release publishing without hosted product CI

## 3. Verification and Publication

- [x] 3.1 Validate shell syntax, guarded failure paths, OpenSpec artifacts, and the complete release build
- [ ] 3.2 Commit and push the release workflow from the remote default branch
- [ ] 3.3 Publish `v0.1.0`, verify both assets and checksums from GitHub, and confirm the stable latest-download URL
