## 1. Common bridge contract

- [x] 1.1 Add a normalized immutable update type and common payload builder
- [x] 1.2 Centralize pane discovery, timestamps, validation, journal-before-publish dispatch, pruning, and snapshot handling
- [x] 1.3 Move the durable store and its tests from the Codex directory into `hooks/common/`
- [x] 1.4 Add focused tests for the common interface and best-effort boundary

## 2. Agent adapters

- [x] 2.1 Migrate Codex lifecycle mapping and completion notification to the common interface without changing reviewer, watcher, or forwarding behavior
- [x] 2.2 Migrate Claude lifecycle mapping and session cleanup to the common interface without changing terminal notification output
- [x] 2.3 Remove duplicated payload construction and Zellij publication from agent-specific entrypoints

## 3. Installation and documentation

- [x] 3.1 Install the common runtime and store beside both global agent entrypoints
- [x] 3.2 Update README, DEVELOPMENT, test tasks, and troubleshooting guidance
- [x] 3.3 Regenerate and review OpenWiki after implementation stabilizes

## 4. Verification

- [x] 4.1 Run focused common, Codex, and Claude Python suites during implementation
- [x] 4.2 Verify installed source/common files and snapshot compatibility
- [x] 4.3 Verify real Codex and Claude lifecycle publication remains pane-isolated inside Zellij
- [x] 4.4 Run `mise run check` and `git diff --check`

## Verification evidence

- `mise run test` passed 58 Rust tests, 16 initial common bridge/store tests, 19 Codex adapter tests, and 11 Claude adapter tests; the colocated-install test subsequently increased common coverage to 17 tests.
- Installed `agent_bridge.py` and `status_store.py` were byte-identical beside both global adapters, both installed snapshot entrypoints returned the same server snapshot, and the installed Claude bridge retained exact `terminalSequence` BEL output.
- Codex 0.145.0-alpha.18 and Claude Code 2.1.216 completed real prompts in separate panes of disposable Zellij session `zvt-common-bridge`; the common host journal recorded independent `done` records for `terminal_0` and `terminal_1` under the same Zellij server.
- The final `mise run check` passed 58 Rust tests, 17 common bridge/store tests, 19 Codex adapter tests, 11 Claude adapter tests, Clippy with warnings denied, the debug WASM build, strict OpenSpec validation, and `git diff --check`.
