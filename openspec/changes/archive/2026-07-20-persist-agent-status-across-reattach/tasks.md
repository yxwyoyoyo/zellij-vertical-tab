## 1. Host lifecycle journal

- [x] 1.1 Add a dependency-free per-server, per-pane journal with strict payload validation, timestamp ordering, matching-session clear behavior, advisory locking, and atomic mode-0600 replacement
- [x] 1.2 Detect the Zellij server ancestor and persist lifecycle, notification, and exit-watcher payloads before normal pipe publication
- [x] 1.3 Add a bounded snapshot command that returns valid version-1 records for a requested positive Zellij server PID
- [x] 1.4 Add Python tests for stale ordering, pane reuse, detached clear tombstones, concurrent writers, malformed files, and snapshot filtering

## 2. Plugin persistence and recovery

- [x] 2.1 Add server-scoped per-instance plugin snapshot files under `/cache` with atomic writes and bounded multi-file restore
- [x] 2.2 Restore lifecycle records and acknowledgements on load without restoring a focus baseline, and persist every later state mutation or pane cleanup
- [x] 2.3 Request `RunCommands`, invoke the fixed global snapshot helper once after permission grant, and strictly validate contextual command results
- [x] 2.4 Merge host recovery through existing snapshot ordering and peer convergence without allowing older recovery to replace newer live state
- [x] 2.5 Add Rust tests for cache isolation, corrupt/oversized entries, acknowledgement restoration, clear reconciliation, and live-versus-recovery ordering

## 3. Installation and documentation

- [x] 3.1 Update global hook installation instructions to include the shared journal module and the new Zellij permission
- [x] 3.2 Document detach, session-switch, reattach, hot-reload, failure fallback, and first-event migration behavior in README and OpenWiki
- [x] 3.3 Regenerate OpenWiki and verify recovery claims against the implementation and OpenSpec delta

## 4. Verification and deployment

- [x] 4.1 Run focused Python and Rust tests while implementing each recovery layer
- [x] 4.2 Run `mise run check`, strict OpenSpec validation, and `git diff --check`
- [x] 4.3 Build and install the release plugin and updated global Codex bridge files
- [x] 4.4 Verify in disposable Zellij sessions that done, acknowledged idle, working, waiting, and detached clear survive or reconcile across detach and reattach without cross-session leakage
- [x] 4.5 Approve the newly requested `RunCommands` permission in `Hub`, then confirm its hot-reloaded sidebar resumes normal live status publication

## Verification evidence

- `mise run check` passed 52 Rust tests, 14 Python tests, Clippy with warnings denied, the `wasm32-wasip1` debug build, strict validation of all OpenSpec artifacts, and `git diff --check`.
- `mise run release` built the optimized WASM; the release artifact and all three global Codex bridge modules were installed, then the installed plugin was hot-reloaded into `Hub`. After `RunCommands` approval, a live `working` update rendered on `terminal_1` and its matching clear removed the badge.
- In the disposable `arcadian-quasar` session, a visible `done` badge survived detach/reattach from plugin cache; an updated lifecycle hook invoked while detached reconciled it to `waiting`; a detached watcher clear removed the badge; and an exact focus-acknowledged completion restored as `idle` after a later detach/reattach.
- A second fresh server using the same `terminal_0` pane ID initially rendered no leaked state, then preserved a `working` badge across detach/reattach. Both disposable sessions and the first server's test journal were removed after testing. Server-PID isolation, newer-live-versus-older-recovery ordering, and malformed/oversized recovery inputs are additionally covered by the focused Rust and Python suites.
