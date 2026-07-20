## Context

Codex hooks publish timestamped lifecycle records keyed by terminal pane ID. The plugin stores those records in every sidebar instance and derives the displayed badge directly from the stored state. Zellij already supplies active-tab and focused-pane metadata through `TabUpdate` and `PaneUpdate`, but focus currently affects row styling only and never acknowledges a completed result.

Focus acknowledgement is local UI state, not a Codex lifecycle event. Replacing a stored `done` record with a fabricated `idle` update would blur that distinction, require the plugin to create timestamps compatible with hook timestamps, and allow peer snapshots or equal-timestamp updates to restore stale presentation.

## Goals / Non-Goals

**Goals:**

- Show `idle` after the user focuses a pane whose current lifecycle state is `done`.
- Keep a newly completed result visible as `done` until the user focuses that pane after completion.
- Keep every sidebar instance consistent, including instances created after acknowledgement.
- Automatically reveal any newer lifecycle state without requiring another focus transition.
- Preserve the existing external hook protocol and timestamp ordering rules.

**Non-Goals:**

- Do not acknowledge or hide `working` or `waiting` on focus.
- Do not change Codex hook mappings, native BEL behavior, or Zellij's bell-clearing lifecycle.
- Do not persist acknowledgement across Zellij sessions or plugin restarts beyond peer snapshot recovery.

## Decisions

### Track acknowledgement against an exact lifecycle record

Store a per-pane acknowledgement key containing the record's session ID and `updated_at_ms`. Rendering returns `idle` only when the current record is `done` and exactly matches the acknowledgement key. A different session or timestamp therefore invalidates the acknowledgement implicitly, and stale acknowledgements cannot hide newer work.

This is preferred to mutating `AgentRecord.state` because lifecycle records remain an unmodified account of hook input. It is also preferred to a boolean flag because a boolean could incorrectly carry across later events or pane reuse.

### Acknowledge only on confirmed focus transitions

After `TabUpdate` or `PaneUpdate`, derive viewed tabs from `other_focused_clients`, then select each viewed tab's focused terminal pane. Fall back to `TabInfo.active` only when Zellij reports no attached-client focus metadata. This distinction matters because every sidebar instance can report its own containing tab as locally active even while the attached client is viewing another tab. Compare that pane set with the last complete focus observation and acknowledge a current `done` record only for a pane newly entering the set. The first complete observation establishes a baseline without acknowledging anything, so startup and hot reload are not treated as user focus actions.

Status and snapshot ingestion never acknowledges focus. This is necessary because a Codex completion pipe can arrive before the `TabUpdate` for a recent tab switch; reconciling against the cached tab would immediately turn an unseen result idle. A completion that arrives while its pane remains focused therefore stays visibly `done` until the user leaves and returns, which preserves the answer-ready signal and makes acknowledgement unambiguous.

Each tab owns a separate sidebar plugin instance, and an inactive instance is not guaranteed to observe the focus update that records leaving its tab. Whenever an instance obtains a changed complete focus observation, it therefore sends the viewed terminal-pane set to its known peers. Recipients replace their focus baseline and may acknowledge a newly focused exact `done` record. A newly discovered peer also receives the sender's current baseline. Focus messages are not forwarded, preventing loops; any resulting exact acknowledgement continues to use the acknowledgement synchronization path.

### Use dedicated internal acknowledgement and focus messages

Add an internal peer message whose payload identifies the acknowledged pane, session, and lifecycle timestamp. Peer application is idempotent and does not forward the message again. Extend synchronization snapshots with acknowledgement keys so newly created sidebar instances recover the same visible state.

Add a second validated peer message containing the current client-viewed terminal-pane IDs. It carries observation state only, not lifecycle state, and is emitted only when the local observation changes or a new peer is discovered. This makes focus history session-wide even when Zellij delivers different event subsets to sidebar instances in active and inactive tabs.

The external `vertical-tab-agent-status` payload remains version 1 and unchanged. Reusing an `idle` lifecycle payload was rejected because it would mix user acknowledgement with Codex-originated status and timestamp ordering.

### Clean up acknowledgement state with pane lifecycle

Remove acknowledgement keys when their terminal pane disappears. Acknowledgements that no longer match a current record may also be pruned after a newer status is accepted; correctness does not depend on pruning because rendering requires an exact match.

## Risks / Trade-offs

- [A pane completes while it remains focused] → Keep `done` visible until a later focus transition; this avoids clearing an answer because status delivery raced focus metadata.
- [Duplicate focus events generate duplicate peer messages] → Make acknowledgement insertion idempotent and broadcast only when local acknowledgement state changes.
- [Peer acknowledgement arrives before its lifecycle record] → Retain the keyed acknowledgement and activate it only if a matching `done` record arrives later.
- [Existing attention-notification work also touches status rendering and synchronization] → Implement against the current worktree carefully and preserve its native bell behavior; validate the combined result with the complete repository gate.

## Migration Plan

No data migration or hook reinstall is required. Build and hot-reload the updated plugin, then create or switch tabs so all sidebar instances run the same wasm. Rollback consists of restoring the previous wasm; acknowledgement state is in memory only.

## Open Questions

None. The intended acknowledgement boundary is Zellij pane focus, and only `done` is converted to visible `idle`.
