## Why

Codex emits `PermissionRequest` after `PreToolUse`, including when automatic approval reviews the request. Treating every permission event as a user wait leaves an automatically approved tool displayed as `waiting` while Codex is actively running it. Some unified-exec paths also defer `PostToolUse` until a long-running command fully exits, so that event alone cannot recover the visible state promptly.

## What Changes

- Keep `working` for permission events whose matching turn context identifies automatic review, with a conservative waiting fallback when that optional context is unavailable.
- Publish `working` when `PostToolUse` confirms that an approved tool finished and control returned to the agent.
- Carry optional lifecycle-event and turn identity in the existing version-1 payload.
- Treat `done` as terminal for its turn so delayed tool hooks cannot overwrite completion.
- Preserve compatibility with legacy version-1 messages and cached records that omit the new optional fields.
- Document and test automatic-review detection, manual approval, fallback behavior, new-turn restart, and delayed-event ordering.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Complete the approval lifecycle and make status ordering aware of turn boundaries.

## Impact

- `hooks/codex/`: lifecycle mapping, hook configuration, host-journal ordering, and Python tests.
- `src/main.rs`: optional lifecycle metadata, turn-aware ordering, cache/snapshot compatibility, and Rust tests.
- README, OpenWiki, and OpenSpec: lifecycle semantics and troubleshooting guidance.
