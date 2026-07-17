## 1. Protocol and State Model

- [x] 1.1 Add serde dependencies and define the versioned status payload and supported states
- [x] 1.2 Implement strict payload parsing, pane ID validation, clear handling, and stale-message rejection
- [x] 1.3 Subscribe to pane updates, map terminal panes to tabs, and remove records for closed panes

## 2. Aggregation and Rendering

- [x] 2.1 Aggregate tracked panes per tab using the specified state precedence and total count
- [x] 2.2 Add prefix-free right-aligned badges while preserving row width, truncation, padding, and selected styling
- [x] 2.3 Preserve existing clicking, scrolling, active-tab following, and startup selectability behavior

## 3. Codex Hook Integration

- [x] 3.1 Add a dependency-free Codex hook bridge that maps lifecycle events to Zellij status messages and no-ops outside Zellij
- [x] 3.2 Add user-level Codex hook configuration for session, prompt, tool, permission, and stop events
- [x] 3.3 Document hook trust, badge meanings, multi-pane aggregation, permissions, and process-exit cleanup behavior

## 4. Verification

- [x] 4.1 Add host unit tests for parsing, state replacement, stale events, cleanup, aggregation, precedence, and badge formatting
- [x] 4.2 Run host tests, strict OpenSpec validation, and the debug and release WASM builds
- [x] 4.3 Run a headless Zellij session with injected multi-pane status messages and verify sidebar behavior

## 5. Global Multi-Project Fix

- [x] 5.1 Correct completed-turn publication semantics and update hook tests and documentation
- [x] 5.2 Install the bridge and hook configuration at user scope without duplicating project-local hook execution
- [x] 5.3 Synchronize status updates and startup snapshots across per-tab sidebar plugin instances, with host tests
- [x] 5.4 Verify concurrent Codex panes from different working directories map to their separate tabs

## 6. Completion and Exit Semantics

- [x] 6.1 Publish `done` from `Stop` and remove the late `PostToolUse` working transition
- [x] 6.2 Start a detached Codex process watcher that clears only its matching session on exit
- [x] 6.3 Add hook and Rust regression tests for done, exit cleanup, and stale-session clear rejection
- [x] 6.4 Build, install, and verify background-tab completion plus live-shell cleanup

## 7. Special Workflow Completion

- [x] 7.1 Add an `agent-turn-complete` notification bridge that publishes `done`
- [x] 7.2 Preserve and test forwarding to the existing external notifier
- [x] 7.3 Install the notifier configuration and verify a background code-review completion

## 8. Artifact Consistency

- [x] 8.1 Fit rows by terminal-cell width so wide Unicode names preserve the status suffix
- [x] 8.2 Reconcile proposal, design, specs, and tasks with done, notifier, exit-watcher, peer-sync, and tombstone behavior
