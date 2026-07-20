## 1. Acknowledgement state model

- [x] 1.1 Add a validated per-pane acknowledgement key containing the Codex session ID and acknowledged lifecycle timestamp
- [x] 1.2 Make rendered status resolve a matching acknowledged `done` record to `idle` while leaving `working`, `waiting`, and unmatched records unchanged
- [x] 1.3 Remove acknowledgement entries for panes removed by `PaneUpdate` and prune entries superseded by newer lifecycle records

## 2. Focus reconciliation

- [x] 2.1 Derive client-viewed tabs from attached-client focus metadata, fall back to active-tab state only when necessary, and resolve their focused terminal panes after both `TabUpdate` and `PaneUpdate`
- [x] 2.2 Track complete focus observations and acknowledge an exact `done` record only when its pane newly gains client-viewed focus, broadcasting only when acknowledgement state changes
- [x] 2.3 Keep status and snapshot ingestion separate from acknowledgement so stale cached focus cannot clear a result before a delayed tab update arrives
- [x] 2.4 Synchronize changed focus observations across per-tab sidebar instances so leaving and returning are retained session-wide

## 3. Sidebar synchronization

- [x] 3.1 Add dedicated internal acknowledgement and focus payloads with strict validation and loop-free peer application
- [x] 3.2 Extend peer snapshots to include acknowledgement references without changing the external version 1 Codex hook payload
- [x] 3.3 Handle acknowledgement-before-status ordering idempotently and recover acknowledged presentation in newly created sidebar instances

## 4. Verification and documentation

- [x] 4.1 Add host unit tests for focus-edge acknowledgement, cached-focus races, cross-instance focus observations, plugin-local versus attached-client tab focus, unaffected working/waiting states, newer-record invalidation, and pane cleanup
- [x] 4.2 Add synchronization tests for malformed payloads, duplicate messages, acknowledgement-before-status ordering, and snapshot recovery
- [x] 4.3 Update README source documentation and regenerate OpenWiki pages to describe focus acknowledgement and its separation from native bell state
- [x] 4.4 Run `mise run check`, strict OpenSpec validation, and `git diff --check`
- [x] 4.5 Build, install, and hot-reload the release plugin in `Hub`
- [x] 4.6 Interactively verify keyboard focus, tab switching, pane-row clicks, completion while focused, and completion while unfocused
