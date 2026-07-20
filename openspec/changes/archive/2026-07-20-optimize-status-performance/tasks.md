## 1. Watcher and hook overhead

- [x] 1.1 Deduplicate exit watchers with a Zellij-server and Codex-PID advisory lock
- [x] 1.2 Use event-driven process-exit observation with a portable bounded polling fallback
- [x] 1.3 Resolve Codex and Zellij ancestors in one traversal during session start
- [x] 1.4 Prune journal directories only for demonstrably dead Zellij PIDs
- [x] 1.5 Add Python tests for duplicate starts, watcher fallback, combined ancestry, and safe cleanup

## 2. Plugin message scaling

- [x] 2.1 Stop relaying lifecycle updates already broadcast by the Zellij CLI pipe
- [x] 2.2 Elect the lowest live plugin ID to fan out focus and acknowledgement synchronization, with one-hop nonleader reports
- [x] 2.3 Preserve late-joining peer snapshot recovery and add leader-turnover/scaling tests

## 3. Persistent cache scaling

- [x] 3.1 Store new plugin snapshots in server-scoped cache directories
- [x] 3.2 Migrate current compatible flat snapshots on first updated restore
- [x] 3.3 Add tests proving historical server entries do not affect normal restore

## 4. Verification and documentation

- [x] 4.1 Add or update performance measurements for watcher memory, hook latency, and message-count complexity
- [x] 4.2 Update maintainer documentation with performance invariants and troubleshooting guidance
- [x] 4.3 Run focused Python/Rust tests, `mise run check`, strict OpenSpec validation, and `git diff --check`
- [x] 4.4 Build/install the release plugin and hooks, then verify multi-tab status, focus acknowledgement, detach recovery, and repeated SessionStart behavior
