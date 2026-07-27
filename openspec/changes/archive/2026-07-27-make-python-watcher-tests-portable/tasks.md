## 1. Portable Watcher Tests

- [x] 1.1 Replace host `select` attribute patches with a complete module-level mock for the kqueue branch
- [x] 1.2 Make the polling test explicitly model a platform without kqueue attributes
- [x] 1.3 Preserve assertions for event construction, queue lifecycle, bounded polling, and sleep behavior

## 2. Documentation and Verification

- [x] 2.1 Document event-driven macOS behavior, supported Unix polling fallback, and the Windows boundary
- [x] 2.2 Run all Python suites and the complete project gate
- [x] 2.3 Regenerate OpenWiki and validate the OpenSpec change
