## Why

The Codex watcher runtime already falls back to polling on platforms without `kqueue`, but its unit tests patch macOS-only `select` attributes directly. On Linux those attributes do not exist, so the test suite fails before it can verify either branch.

## What Changes

- Mock the watcher module's complete `select` dependency so the event-driven and polling branches can be exercised on both macOS and Linux.
- Preserve the existing runtime behavior: use kqueue when available and bounded-interval polling otherwise.
- Document the Unix portability boundary and validate the Python suites independently of host `select` capabilities.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Specify the existing polling behavior when process-exit events are unavailable and require portable coverage of both watcher branches.

## Impact

The change affects the Codex watcher tests, the agent-status specification, and testing documentation. It does not change the hook protocol, watcher runtime, release assets, or the existing Unix dependency on `fcntl`.
