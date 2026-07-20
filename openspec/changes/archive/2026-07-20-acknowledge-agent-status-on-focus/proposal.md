## Why

A completed Codex turn remains marked `done` after the user returns to its pane, so the sidebar continues to present already-seen work as outstanding. Pane focus should acknowledge a completed result while preserving statuses that still describe active work or required input.

## What Changes

- Treat focus of a terminal pane as acknowledgement of that pane's current `done` status.
- Present an acknowledged completed session as `idle` without rewriting the timestamped lifecycle record received from Codex.
- Require a confirmed focus transition after completion, so delayed tab metadata cannot acknowledge a result in an unseen pane.
- Synchronize the last client-viewed pane set across per-tab sidebar instances so leaving a tab and returning is observed session-wide.
- Synchronize acknowledgement across all sidebar plugin instances and expire it when a newer lifecycle update arrives.
- Keep `working`, `waiting`, and `clear` behavior unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Add focus-driven acknowledgement semantics for completed Codex turns, including cross-instance synchronization and reset on newer lifecycle state.

## Impact

- `src/main.rs`: focus transition handling, acknowledgement state, status rendering, peer synchronization, and unit tests.
- `openspec/specs/agent-status/spec.md`: lifecycle presentation and synchronization requirements.
- `README.md` and generated OpenWiki documentation: explain that returning to a completed pane changes its visible badge to idle.
- No protocol change is required for the existing Codex hook payloads.
