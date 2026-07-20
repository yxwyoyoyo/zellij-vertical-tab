## Context

The current bridge maps `PreToolUse` to `working` and `PermissionRequest` to `waiting`. Codex fires the permission request after the pre-tool event, including when automatic review will decide it. In a real unified-exec turn, `PostToolUse` was not emitted when automatic review approved the request and the long-running command began, so the pane remained waiting while Codex itself displayed working.

Codex exposes `turn_id` and `transcript_path` on turn-scoped hooks but no separate approval-resolved lifecycle hook. The transcript's matching `turn_context` currently identifies `approvals_reviewer` as `auto_review` or `user`; the transcript format is not a stable hook interface, so this signal must be bounded, best-effort, and conservative on failure. `PostToolUse` still proves an approved tool finished and the main agent regained control. External `agent-turn-complete` notifications may omit turn identity, so completion protection must also have a safe event-based fallback.

## Goals / Non-Goals

**Goals:**

- Keep automatic-review permission events working without hiding genuine manual waits.
- Return `waiting` to `working` after a manually approved tool completes.
- Keep `done` terminal against delayed events from the completed turn.
- Allow a new user prompt to begin work after `done`.
- Apply identical ordering in the host journal and WASM plugin.
- Read legacy version-1 live messages, journal entries, and plugin cache snapshots.

**Non-Goals:**

- Do not claim that Codex exposes an approval-result hook.
- Do not infer automatic review when the matching reviewer context is unavailable.
- Do not change badge appearance, focus acknowledgement, or notification behavior.

## Decisions

### Distinguish automatic review at PermissionRequest

For `PermissionRequest`, inspect only the bounded tail of `transcript_path` and select a `turn_context` whose `turn_id` exactly matches the hook. If its `approvals_reviewer` is `auto_review`, publish working because no user response is required. If the transcript, turn, field, or value is unavailable, publish waiting. A manual reviewer therefore retains the existing waiting behavior.

This uses an explicitly unstable convenience interface only as an optional enhancement: malformed records are skipped, I/O errors do not affect Codex, reads are capped at 8 MiB, and the safe fallback remains waiting. Exact turn matching prevents another concurrent or previous turn from determining the state.

### Use PostToolUse as the return-to-agent boundary

Register the existing lifecycle bridge for `PostToolUse` and map it to `working`. A manual approval remains `waiting` while the user decides. Once the approved tool finishes, `PostToolUse` changes the status back to `working` while the agent reasons or selects its next action. The same event is harmless for an auto-reviewed request that already remained working.

This is preferred to a timer because tool and review duration are unbounded. It is preferred to inspecting terminal output because output is not a stable lifecycle interface.

### Extend version 1 with optional lifecycle metadata

Add optional `event` and `turn_id` fields to status records. Hook events use normalized snake-case event names; `turn_id` is copied only when it is a non-empty string. Notification completion and process-exit clear records include an event but tolerate absent turn identity.

Keeping the fields optional under version 1 provides rolling compatibility: old readers ignore them, while new readers accept legacy records without them. Persistent cache and host snapshots retain the fields when present.

### Make done terminal within a turn

For the same pane and session, a newer record cannot replace `done` when both records name the same turn. When either turn identity is unavailable, only `UserPromptSubmit` may reopen a completed session; delayed `PreToolUse`, `PermissionRequest`, or `PostToolUse` remains rejected. A record with a distinct known turn may proceed normally.

This fallback protects completion emitted by the external notifier, whose payload may lack `turn_id`, while preserving the normal new-turn transition. Timestamp ordering remains the first stale-record guard, and matching-session clear behavior remains unchanged.

### Keep host and plugin merging equivalent

The Python journal and Rust plugin apply the same terminal-done rule. This prevents detach recovery from resurrecting a state that the live plugin would reject, and ensures plugin cache, host snapshot, peer snapshot, and live pipe convergence reach the same result.

## Risks / Trade-offs

- [Transcript format changes or reviewer context is outside the bounded tail] → Fall back to waiting rather than conceal a possible user decision.
- [PostToolUse arrives only after a long-running tool finishes] → Auto-reviewed requests already remain working; manually reviewed requests return to working when the event arrives.
- [A new turn's UserPromptSubmit hook is missing and its turn ID is absent] → Conservatively retain `done` rather than display a false working state.
- [Legacy records omit metadata] → Preserve timestamp behavior except for the conservative terminal-done fallback.
- [Old plugin runs with new hooks during deployment] → Extra optional fields are ignored and normal timestamp behavior continues until the plugin is updated.

## Migration Plan

Install the updated hook configuration and bridge files, then deploy the plugin. Existing version-1 cache and journal files remain readable. The next lifecycle event adds metadata to the current pane record. Rollback is compatible because the payload version and required fields do not change.

## Open Questions

None.
