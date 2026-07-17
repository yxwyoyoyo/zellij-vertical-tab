## Context

The plugin currently receives only tab and mouse events. `TabInfo` has no knowledge of processes inside terminal panes, so accurate Codex state cannot be inferred from the existing model. Zellij 0.44.3 provides both `PaneUpdate(PaneManifest)` for mapping terminal pane IDs to tabs and the `pipe()` lifecycle method for receiving arbitrary CLI messages. Codex provides global and project-local lifecycle hooks with JSON input and preserves the surrounding Zellij environment, including `ZELLIJ_PANE_ID`.

The plugin is instantiated once per tab from `default_tab_template`. CLI pipe delivery to inactive instances is not reliable, so sidebar instances explicitly synchronize their session-wide state. Existing startup constraints remain mandatory: the crate stays a WASI binary, `zellij-tile` must match the Zellij binary, selectability is deferred until the first event, and layout children stay wrapped in a sibling pane.

## Goals / Non-Goals

**Goals:**

- Show Codex states beside the owning tab without an agent-name prefix, distinguishing a new session (`idle`) from a completed answer (`done`).
- Publish status for Codex sessions launched from any working directory through one global integration installation.
- Track the current Codex session independently for every terminal pane.
- Aggregate multiple tracked panes in one tab using a deterministic dominant state and total count.
- Reject malformed, unsupported, and out-of-order messages without disrupting Codex or Zellij.
- Remove state when its terminal pane disappears and preserve every existing sidebar interaction.

**Non-Goals:**

- Detect Claude Code or other agents in this change.
- Display Codex subagents separately from their top-level terminal pane.
- Inspect terminal output or transcript files.
- Add an expanded per-pane status screen.

## Decisions

### Use globally installed Codex hooks, notifications, and a Zellij pipe

Two Python scripts installed under `~/.codex/hooks/` consume lifecycle-hook JSON on standard input and completion-notification JSON as a command argument. They combine those events with `ZELLIJ_PANE_ID` and broadcast versioned JSON payloads through a named Zellij pipe. User-level `~/.codex/hooks.json` and `notify` configuration make the bridges available to Codex sessions in every project; the repository retains source copies for versioning and installation. This uses documented lifecycle boundaries and avoids parsing terminal UI text or unstable transcript files. Python's standard library keeps both bridges dependency-free and allows status-publication failures outside Zellij to remain non-blocking.

`Stop` maps to `done`, making a newly delivered answer visually distinct from an untouched idle session. `PostToolUse` does not publish state: `UserPromptSubmit` and `PreToolUse` already establish `working`, while a late `PostToolUse` process could otherwise overwrite the terminal `Stop` state.

Codex has no session-exit lifecycle hook. On `SessionStart`, the bridge therefore locates the nearest Codex ancestor process and starts a detached watcher. When that process exits, the watcher publishes `clear` for the original pane and session. The plugin only accepts a clear whose session ID still matches the pane's current record, so an older watcher cannot clear a newer Codex session after pane reuse.

Some special Codex workflows, notably interactive code review in Codex 0.144.5, can complete without invoking the `Stop` hook even though the rollout reaches `task_complete`. The user-level Codex `notify` command supplies an independent `agent-turn-complete` signal, so a second small bridge publishes `done` from that notification. The bridge also forwards the unmodified notification payload to the user's pre-existing notifier command, preserving Computer Use integration. Duplicate `done` messages from normal turns are harmless because they name the same pane and session.

Alternatives considered:

- Parsing pane titles is lower setup but cannot represent all lifecycle states reliably.
- Reading pane output requires a broader permission and couples behavior to Codex rendering details.
- Polling process trees from the WASI plugin does not reveal approval or turn completion; the external bridge uses only a narrow process-exit watcher for cleanup.

### Keep one current record per terminal pane

The plugin stores `pane_id -> {session_id, state, updated_at_ms}`. A newer message replaces the record even when the session ID changes; an older timestamp is ignored. A `clear` is retained internally as a non-rendered tombstone until the pane closes or a newer session arrives, preventing delayed sync updates or snapshots from resurrecting an exited session. One terminal pane cannot host two foreground Codex sessions concurrently, while separate panes naturally receive separate records. This prevents resumed or restarted sessions from inflating tab counts.

### Map pane records to tabs from `PaneUpdate`

`PaneManifest.panes` is reduced to a terminal-pane-to-tab-position map. Every update removes status records for panes no longer present. Records received before the first pane update may be retained until the manifest arrives, allowing startup races to converge.

### Aggregate at render time

For every tab, non-cleared pane records produce a count and dominant state. Clear tombstones are retained for ordering but excluded from aggregation. State precedence is `waiting > working > done > idle`, because approval/input needs attention first, active work next, and quiescent states last. A single pane renders only the glyph; multiple panes append the total count, such as `?2`. Agent identity is intentionally absent from the display.

### Synchronize per-tab plugin instances

`default_tab_template` creates one sidebar plugin instance per Zellij tab, and inactive instances do not reliably receive every CLI pipe message. Each instance records its own plugin ID, discovers sibling instances with the same plugin URL from `PaneUpdate`, and uses targeted Zellij plugin messages for synchronization. An instance that receives an external status applies it and forwards a distinct sync-update message to its peers. A newly discovered instance requests a full snapshot; snapshot records are merged through the same per-pane timestamp checks. Sync-update and snapshot messages are never forwarded again, preventing loops.

### Reserve a right-aligned suffix

Row formatting receives an optional badge. When present, it reserves the badge's terminal-cell width plus a separating cell, truncates/pads the existing index-and-name body by terminal-cell width, and appends the badge. This preserves the exact pane width and full-row selected styling even for wide Unicode tab names. Rows without a badge retain their current format.

### Use a versioned JSON protocol

The initial protocol is version `1` and includes `pane_id`, `session_id`, `state`, and `updated_at_ms`. JSON gives explicit validation and future extension. `serde` and `serde_json` become direct dependencies; they are already transitive dependencies of the Zellij stack, so the incremental compatibility risk is small.

## Risks / Trade-offs

- **Hook trust is required before user-level commands run** → document the one-time Codex `/hooks` review step and make the script harmless outside Zellij.
- **Permission prompts change because `ReadCliPipes` and `MessageAndLaunchOtherPlugins` are new** → keep both permissions narrowly scoped and cover the first-launch prompt in runtime verification.
- **Pipe events can race across panes** → compare per-pane millisecond timestamps and ignore older records.
- **Codex has no session-exit hook** → start a best-effort detached process watcher from `SessionStart`; pane-close cleanup remains the fallback if ancestor discovery fails.
- **Some workflows omit `Stop`** → also consume the documented external `agent-turn-complete` notification and preserve any existing notifier through forwarding.
- **Unicode glyphs and wide tab names consume variable terminal width** → calculate display-cell width with `unicode-width` and cover wide names plus narrow panes.
- **Broadcast messages reach unrelated plugins** → use a unique pipe name; unrelated plugins ignore it, and this plugin rejects every other name.
- **Each tab owns a separate plugin instance** → request `MessageAndLaunchOtherPlugins`, target only discovered sibling plugin IDs, and synchronize updates plus startup snapshots without rebroadcast loops.

## Migration Plan

1. Add the OpenSpec delta requirements and tests before enabling the hook.
2. Add pipe and pane tracking, then badge rendering.
3. Add the lifecycle and completion-notifier bridges plus configuration template under `hooks/codex/`, install equivalent user-level files under `~/.codex`, and merge the notifier into `~/.codex/config.toml` without replacing an existing notifier.
4. Build the host tests and WASM module, then launch the development layout and accept the new pipe permission.
5. Review and trust the global hook in Codex, then verify two Codex sessions from different working directories, inactive-tab completion, code-review completion, and process-exit cleanup.

Rollback restores any previous `notify` command, removes the lifecycle hook configuration and both bridge scripts, then reverts the plugin changes. Without status messages, the enhanced plugin renders the original tab rows.

## Open Questions

None for this change. Support for other agent products remains out of scope.
