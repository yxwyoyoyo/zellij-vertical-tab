## 1. Claude lifecycle bridge

- [x] 1.1 Add a dependency-free Claude Code bridge for session, prompt, tool, permission, completion, and session-end events
- [x] 1.2 Normalize Claude prompt identity into the existing turn-aware protocol and journal every valid event before pipe publication
- [x] 1.3 Add a user-level Claude settings template covering all supported event groups
- [x] 1.4 Add focused tests for lifecycle mapping, malformed/outside-Zellij behavior, persistence ordering, pipe publication, and matching-session clear

## 2. Recovery and shared protocol

- [x] 2.1 Extend the shared event validator for normalized Claude continuation events
- [x] 2.2 Let the plugin's fixed recovery command fall back to the Claude snapshot helper when the Codex helper is absent
- [x] 2.3 Add Rust coverage for recovery-helper precedence and fallback command safety

## 3. Installation and documentation

- [x] 3.1 Install the Claude bridge and shared store under `~/.claude/hooks`
- [x] 3.2 Merge the template hook groups into the existing user settings without changing unrelated keys
- [x] 3.3 Update README, DEVELOPMENT, and source OpenSpec language from Codex-only to supported-agent behavior
- [x] 3.4 Regenerate and review OpenWiki after implementation stabilizes
- [x] 3.5 Document Claude native attention and its minimum supported Claude Code version

## 4. Claude native attention

- [x] 4.1 Return a single BEL through Claude's supported `terminalSequence` output for visible permission requests and final answers
- [x] 4.2 Suppress completion attention when `stop_hook_active` shows that Claude is continuing
- [x] 4.3 Add focused tests for attention-event selection and exact hook JSON output

## 5. Verification

- [x] 5.1 Run focused Python, Rust, and strict OpenSpec tests during implementation
- [x] 5.2 Verify a real Claude session moves through working, waiting, done, and clear in a disposable Zellij session
- [x] 5.3 Verify Codex and Claude Code in separate panes retain independent status and detach recovery
- [x] 5.4 Verify permission and completion attention in a real Claude session inside Zellij
- [x] 5.5 Run `mise run check` and `git diff --check`

## Verification evidence

- The initial lifecycle revision passed `mise run check` with 58 Rust tests, 28 Codex bridge tests, and 8 Claude bridge tests before notification coverage was added.
- Claude Code 2.1.216 in disposable session `charming-glockenspiel` published `idle` on startup, `working` on prompt submission, `waiting` at a real Bash permission dialog, `working` after approval, `done` at `Stop`, and `clear` after `/exit`; the host journal and plugin cache matched the live lifecycle.
- Codex 0.144.6 and Claude Code 2.1.216 ran concurrently in separate panes of disposable session `nautical-jellyfish`; both pane records independently reached `working` and `done`, and both restored from the same server-scoped cache after detach and reattach.
- The installed Claude Code 2.1.216 bridge returned BEL through `terminalSequence` for both a completed response and a real Bash `PermissionRequest` in disposable session `zvt-claude-notify`; with the Claude tab inactive, `zellij action list-tabs --json --all` reported `has_bell_notification: true` for each event, and focusing the tab cleared the retained bell between tests.
- The notification revision passed `mise run check`: 58 Rust tests, 28 Codex bridge tests, 11 Claude bridge tests, Clippy with warnings denied, the debug WASM build, strict OpenSpec validation, and `git diff --check`.
