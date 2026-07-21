## Context

The Rust plugin already consumes one agent-neutral version-1 wire protocol, but each Python integration currently constructs that wire payload and invokes Zellij independently. `status_store.py` is semantically common yet lives under `hooks/codex/` and is copied from there for Claude installation. New integrations therefore have no repository-level interface separating native hook parsing from shared transport and recovery behavior.

## Goals / Non-Goals

**Goals:**

- Make a new agent integration responsible only for mapping native events into normalized lifecycle updates and any native hook response.
- Centralize pane discovery, timestamps, payload validation, journal-before-publish ordering, Zellij publication, and snapshot output.
- Preserve the version-1 wire protocol, lifecycle behavior, installed hook commands, and best-effort failure boundary.
- Keep source ownership agent-neutral without requiring a Python package installation.

**Non-Goals:**

- Do not add another supported agent in this change.
- Do not add an agent identity to badges or the wire protocol.
- Do not force every agent to implement Codex's exit watcher, notifier forwarding, or Claude's terminal response.
- Do not change Rust state, rendering, or recovery-helper precedence.

## Decisions

### Normalize before transport

`hooks/common/agent_bridge.py` exposes an immutable `AgentUpdate` containing `session_id`, canonical `state`, canonical `event`, and optional `turn_id`. Adapters create this value from native input. The common runtime obtains `ZELLIJ_PANE_ID`, assigns the timestamp, builds and validates the version-1 payload, finds the Zellij server, optionally prunes dead server journals, persists before publishing, and serves snapshots.

The adapter cannot supply pane identity or timestamps. This keeps environment ownership and ordering consistent across agents and prevents a new adapter from accidentally defining a second transport contract.

### Preserve optional agent extensions

The normalized update is the required interface; native responses and cleanup strategies are optional adapter behavior. Codex retains transcript inspection for auto-review, external notifier forwarding, and its locked process-exit watcher. Claude retains `terminalSequence` JSON for permission and final-stop attention and uses `SessionEnd` for cleanup. Both call the same common dispatch function for status changes.

### Keep deployment dependency-free

Repository source lives under `hooks/common/`. Installation copies `agent_bridge.py` and `status_store.py` beside each enabled agent entrypoint. Each adapter adds the repository common directory to its import search path when run from the checkout, while installed adapters import the colocated copies. No wheel, virtual environment, or global Python package is required.

### Test the contract independently

Common tests verify normalized construction, rejection of invalid updates, journal-before-publication ordering, outside-Zellij behavior, pruning, snapshot output, and publication failure isolation. Agent tests focus on native mapping and extensions, with compatibility assertions that their resulting payloads remain unchanged.

## Risks / Trade-offs

- [Installed common files drift between agents] -> Document copying both common files during every bridge update and compare installed files during verification.
- [Import fallback hides a broken installation] -> Preserve best-effort success for agent safety, but cover source and installed layouts with tests and troubleshooting guidance.
- [Common interface grows around one agent's special cases] -> Keep watchers, notifier forwarding, and terminal responses outside the required `AgentUpdate` contract.

## Migration Plan

Move the repository store and tests into `hooks/common/`, install both common modules beside the existing Codex and Claude entrypoints, and leave user hook configuration unchanged. Rollback restores the previous colocated store source and adapter-owned dispatch functions without changing persisted journals or the wire protocol.

## Open Questions

None.
