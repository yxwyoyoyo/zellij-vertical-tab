---
type: Architecture Guide
title: Plugin Architecture and Domain Model
description: Runtime architecture of the pane-aware Zellij sidebar and common Python agent bridge, covering native nested-list rendering, pane-owned Codex and Claude Code status, synchronization, bounded same-server recovery, input, and layout constraints.
resource: src/main.rs
tags: [architecture, zellij, rust, wasm, codex, claude-code, panes]
---

# Plugin architecture and domain model

The plugin is one Rust binary compiled to `wasm32-wasip1`. Zellij owns tabs, panes, focus, dimensions, theme resolution, native bell state, and application mutations; the plugin retains presentation state, builds a deterministic hierarchy, and maps visible rows back to Zellij targets. The [quickstart](quickstart.md) gives product and source orientation, while [development and operations](development.md) turns these constraints into repeatable workflows.

## Runtime boundary and lifecycle

```text
TabUpdate ───────────────> tabs / active tab
PaneUpdate ──────────────> pane ownership + ordered terminal metadata + peers
Codex/Claude adapters ──AgentUpdate──> common bridge ──host journal + Zellij broadcast pipe──> pane-keyed AgentRecord
                                                    server-sharded plugin /cache <──── runtime restart ────────┘
Focus changes ──nonleader report──> lowest-ID leader ──bounded fanout──> sidebar peers
Codex TUI / Claude hook output ──BEL> Zellij visual bell ──TabInfo.has_bell_notification──> tab attention icon
                                      │
                                      v
                         flattened Vec<SidebarRow>
                         │        │         │
                         render   scroll    click target
```

`load()` records this sidebar's plugin ID and Zellij server PID; restores matching snapshots from the current server's `/cache` shard, lazily migrating compatible flat snapshots when necessary; requests `ReadApplicationState`, `ChangeApplicationState`, `ReadCliPipes`, `MessageAndLaunchOtherPlugins`, and `RunCommands`; and subscribes to tab, pane, mouse, permission, and command-result events. Once permission is granted, the instance invokes a fixed global helper once to reconcile host-journal events emitted while detached. The first `update()` calls `set_selectable(false)` once. `render()` converts the visible hierarchy into `NestedListItem` values and emits one native list with `print_nested_list_with_coordinates`; `pipe()` validates external agent updates and handles instance-to-instance synchronization (`src/main.rs`).

`TabUpdate` also carries Zellij's persistent bell state. The row model places `` on the owning tab while `PaneUpdate` and the status pipe retain exact per-pane lifecycle ownership. This deliberate split avoids guessing which pane emitted a bell because Zellij exposes retained bell state only on `TabInfo`.

The explicit `[[bin]]` target in `Cargo.toml` is required because Zellij loads a command-style WASM module with `_start`; `register_plugin!(State)` supplies its entrypoint. A non-WASM `host_run_plugin_command` stub keeps pure host-target tests linkable.

## State and ownership

| State | Authority and purpose |
| --- | --- |
| `tabs`, `active_idx` | Latest Zellij tab order and active vector index |
| `pane_tabs` | Stable terminal pane ID to tab-position ownership, used for lifecycle cleanup |
| `terminal_panes` | Terminal display metadata grouped by tab: title, focus, layer, and geometry |
| `agent_records` | Newest `AgentRecord` per terminal pane ID, including non-rendered `clear` tombstones |
| `agent_acknowledgements`, `focused_terminal_panes` | Exact completed records acknowledged by focus and the last complete client-viewed focus observation |
| `plugin_id`, `zellij_pid`, `peer_plugin_ids` | Identity of this sidebar, its live Zellij server, same-URL sidebar instances, and the lowest-ID synchronization leader |
| `host_restore_requested` | One-shot guard for host-journal reconciliation after permission grant |
| `scroll_offset`, `rows` | First flattened hierarchy row and last rendered viewport height |
| `unselectable_set` | One-time guard for deferred selectability |

Pane IDs, not tab names or pane titles, own agent state and pane click identity. Titles are presentation-only and fall back to `pane <id>` when empty. `TabInfo.position` remains zero-based internally and becomes one-based only when passed to `switch_tab_to`; visible labels rely on the native `>` bulletin instead of repeating the position.

## Adaptive tab and pane hierarchy

`terminal_panes_by_tab` filters out plugin panes and orders terminal panes by:

1. tiled before floating before suppressed;
2. vertical position (`pane_y`);
3. horizontal position (`pane_x`);
4. pane ID as a stable tie-breaker.

This is a deterministic visual hierarchy, not a reconstruction of Zellij's split tree: `PaneManifest` exposes a flat list and overlapping layers, so deeper ancestry would be speculative (`openspec/changes/archive/2026-07-17-nest-panes-under-tabs/design.md`).

`build_sidebar_rows` creates the one canonical flattened sequence used by render, scroll, overflow, active-tab following, and mouse input:

- Every tab creates `SidebarRow::Tab`.
- Zero or one terminal pane creates no child.
- Two or more terminal panes create one `SidebarRow::Pane` per ordered terminal.
- An active tab is selected; among its children, only the focused pane in the visible tiled/floating layer is selected.

Because the same row objects expose both presentation state and `RowTarget`, hierarchy changes cannot silently desynchronize click coordinates from rendered rows.

## Per-pane agent status and synchronization

The [Codex](development.md#codex-bridge-installation) and [Claude Code](development.md#claude-code-bridge-installation) adapters map native events into `AgentUpdate`; the [common adapter runtime](development.md#common-agent-adapter-interface) publishes version-1 JSON on `vertical-tab-agent-status` with pane ID, session ID, state, millisecond timestamp, and optional normalized lifecycle event and turn identity. Claude copies a non-empty prompt ID into the protocol turn ID; Codex derives turn identity from its lifecycle context. States are `idle`, `working`, `waiting`, `done`, and `clear`; their visible glyphs use dim, cyan emphasis, orange emphasis, and success styling respectively. Codex uses transcript context to keep auto-reviewed permission requests working, while Claude maps a visible `PermissionRequest` to waiting. Claude's `PostToolUse`, `PostToolUseFailure`, and `PermissionDenied` events return to working. Within one turn or prompt, done is terminal against delayed tool and permission events; a new `UserPromptSubmit` reopens the session as working. Legacy version-1 records without optional lifecycle metadata remain valid.

For Claude Code 2.1.141 or newer, the same command hook returns a single BEL through top-level `terminalSequence` output on `PermissionRequest` and final `Stop`. A `Stop` with `stop_hook_active: true` emits no attention because Claude is continuing. This output does not approve, deny, or block the hook. Zellij owns the resulting tab-scoped native bell while lifecycle status remains pane-scoped.

### Update rules

`parse_agent_status` accepts only the supported version, a terminal pane ID, a non-empty session ID, a recognized state, valid optional lifecycle metadata, and valid JSON. `apply_agent_status` then enforces per-pane current ownership:

- an older timestamp cannot replace a newer record;
- a newer session can replace a prior session in a reused pane;
- auto-reviewed permission requests remain working when reviewer context is available;
- `PostToolUse` replaces waiting with working after an approved tool finishes;
- done rejects delayed events from its turn, while a new prompt or distinct known turn may resume work;
- `clear` is accepted only for the matching current session;
- a retained `clear` tombstone prevents delayed messages from resurrecting state;
- `PaneUpdate` removes records for terminal panes that no longer exist.

Focus acknowledgement is presentation state rather than a lifecycle event. After tab or pane updates, the plugin uses `TabInfo.other_focused_clients` to find tabs actually viewed by attached clients, falling back to `active` only when no client-focus metadata exists. This avoids treating every sidebar instance's locally active containing tab as user-visible. The first complete observation establishes a focus baseline; later observations acknowledge a current `done` record only when its pane newly enters the viewed-focus set. Since an inactive per-tab instance can miss the update that records leaving its tab, changed focus observations are sent to peer instances and applied without forwarding. Status and snapshot ingestion never acknowledges against cached focus, preventing a completion pipe from racing a delayed tab-switch update and clearing an unseen result. Rendering resolves only an exact acknowledged session and timestamp to visible `idle`; working, waiting, clear, different-session, and different-timestamp records retain their lifecycle presentation. Pane cleanup removes acknowledgement references alongside records.

### Row placement replaces aggregation

- **Exactly one terminal pane:** its renderable state appears on the compact tab row.
- **Multiple terminal panes:** the parent tab has no status; each child independently displays only its own pane's state.
- **No terminal or tracked pane:** no badge is rendered.

There is no dominant-state precedence and no pane-count suffix in the current design. This preserves exact ownership when two agent sessions in one tab are in different states. Native attention is separately tab-scoped: a one-pane compact row can show `status bell`, while a multi-pane parent shows the bell and its children retain their own status icons.

### Sidebar instance convergence

`default_tab_template` creates a separate plugin instance in every tab. The untargeted `vertical-tab-agent-status` CLI pipe is already broadcast by Zellij, so each listening instance applies an external lifecycle update locally and **does not relay it**; the legacy sync-update receive path remains compatible with existing senders. On `PaneUpdate`, instances discover plugin panes with the same plugin URL and deterministically elect the lowest live plugin ID as synchronization leader. A nonleader reports a changed focus observation only to that leader; the leader fans focus observations and resulting exact-record acknowledgements to peers. When it closes, the next-lowest ID takes over from the next pane manifest. In the conservative case where every nonleader reports and the leader sends both focus and acknowledgement fanouts, one transition uses at most `3(N - 1)` peer messages for `N` sidebars; the path remains linear rather than all-to-all.

A newly discovered peer still receives a sync request and the sender's current focus baseline; the peer returns a bounded serialized snapshot containing lifecycle records and acknowledgement references, and timestamp validation merges only current state. This snapshot path recovers a sidebar created after the original pipe broadcast. Recipients apply focus and acknowledgement messages without forwarding, so there is no loop. An acknowledgement may arrive before its matching status and becomes visible only after that exact `done` record arrives (`src/main.rs`, `openspec/changes/archive/2026-07-20-optimize-status-performance/design.md`).

### Durable recovery

Agent-specific entrypoints normalize native hook data into `AgentUpdate`, then `hooks/common/agent_bridge.py` obtains the pane ID and timestamp, validates the version-1 payload, persists it through `hooks/common/status_store.py`, and publishes it to Zellij. Records are isolated by Zellij server PID and terminal pane, serialized under an advisory per-pane lock, ordered by timestamp, session, turn, and terminal-completion rules, and replaced atomically. Codex starts one locked PID watcher because it has no session-end hook; Claude Code instead emits a matching-session clear from `SessionEnd`. Session start also prunes only numeric host-journal shards whose PID is demonstrably absent. This host journal remains available when a detached session has no plugin runtime to receive an undirected pipe (`hooks/common/`, `hooks/codex/`, `hooks/claude/`).

Every plugin mutation also serializes lifecycle records and exact acknowledgement references to `/cache/agent-status-<zellij-pid>/agent-status-<plugin-id>.json`. On `load()`, an instance scans only bounded, well-formed files in its current server shard and merges them through the normal timestamp, session, turn, and terminal-completion rules. If that shard does not yet exist, the first updated instance reads compatible legacy flat files for the current server and persists their merged state into the shard; the shard then prevents later restores from rescanning flat history. It deliberately does not restore `focused_terminal_panes`, so runtime startup cannot fabricate a focus transition or acknowledge an unseen completion.

After `RunCommands` is granted, the plugin runs a fixed `agent_status.py --snapshot <zellij-pid>` command once, preferring the installed Codex helper and falling back to the installed Claude helper. Only a successful, bounded UTF-8 result with the matching command context is parsed. Host recovery, cache recovery, peer snapshots, and live pipe updates all enter through the same versioned snapshot/status validators and turn-aware ordering, so arrival path cannot change the resulting lifecycle state. Invalid files, denied permission, helper failure, and unavailable persistence are ignored without disabling in-memory status handling. Server-PID namespacing prevents another live Zellij server with reused pane IDs from inheriting state; persistence is not intended to resurrect a session after its server process exits.

## Rendering and interaction rules

### Native nested-list presentation

Every visible `SidebarRow::Tab` becomes a level-zero native list item and every `SidebarRow::Pane` becomes a level-one item. Zellij supplies the `>` and `-` bulletins, indentation, bold labels, complete-row padding, and `list_selected`/`list_unselected` theme palettes. The `>` is the sole persistent leading marker for a tab, so the label does not repeat its one-based position. The active tab and focused visible pane child set the item's selected flag. Semantic badge ranges are applied to the native item before selection, so idle, working, waiting, done, and native-attention colors remain theme-derived on both selected and unselected rows.

### Width-fitted rows and right inset

The native renderer consumes three cells before level-zero content and five before level-one content. Inside the remaining budget, both tab and pane content reserve one leading cell for an optional overflow marker; native child indentation deliberately places pane titles deeper than tab names. The formatters use `unicode-width` cell measurements, never split a character, and append `…` when the name budget is exceeded.

Native list items are rendered to the exact `cols` width so selected background styling spans the line. `ROW_RIGHT_PADDING = 1` reserves one trailing content cell whenever the required prefix or suffix can still fit. Badge rows reserve the complete right-aligned status and attention suffix first, keep semantic color ranges off the separator and trailing cell, and truncate only the title. Extremely narrow rows prioritize native attention rather than clipping the suffix.

### Viewport and overflow

`clamp_offset` bounds scrolling against the flattened row count. `visible_window` minimally shifts that offset to reveal the active **tab row**; pane children before it therefore count in the coordinate. `▲` marks hidden hierarchy rows above and `▼` hidden rows below. Wheel events move exactly one flattened row.

### Exact pane clicks

A nonnegative mouse line is translated through `scroll_offset` and looked up in the same flattened row vector used for rendering:

- `RowTarget::Tab { position }` calls `switch_tab_to(position + 1)`.
- `RowTarget::Pane { id }` calls `focus_terminal_pane(id, false, false)`; Zellij switches to the owning tab/layer and focuses that stable terminal ID.
- A line without a row is a no-op.

The click itself returns `false`; the resulting `TabUpdate` or `PaneUpdate` drives the rerender.

## Runtime and layout constraints

`zellij.kdl` uses `default_tab_template` to place a borderless sidebar with flexible `size="13%"` beside `pane { children }`, with `zellij:status-bar` below and no horizontal tab bar. The percentage approximates the former 32 columns on the tested 245-column viewport, but unlike a fixed integer dimension it permits Zellij's tiled grid to resize the boundary. New tabs inherit the template and create new synchronized sidebar instances at the configured initial percentage.

The plugin does not implement dragging or persist geometry. With Zellij mouse handling enabled (the default), the one-cell tiled boundary between the sidebar and content is the native drag handle; the sidebar itself remains borderless and unselectable. Pane frames are optional: showing them draws the boundary as the content pane's left frame, while hiding them leaves the same hit target. `advanced_mouse_actions` is not required for resizing. Zellij owns subsequent width per tab, so resizing one tab does not synchronize another tab and a new tab starts at 13%.

Two crash-derived constraints stabilize the layout on Zellij 0.44:

- `set_selectable(false)` must be deferred from `load()` to the first event.
- `children` must remain wrapped in its own pane beside the unselectable plugin pane.

Layout geometry is created when Zellij builds the tab. Hot reload cannot convert an existing fixed layout or restore the initial percentage, so layout and drag changes require a fresh-session regression pass from [development and operations](development.md#runtime-verification).

## Integration points and safe changes

- **Zellij ABI:** `zellij-tile = 0.44.3` supplies lifecycle, pane metadata, pipes, peer messages, rendering, and focus/switch APIs.
- **Common agent bridge:** `hooks/common/agent_bridge.py` defines `AgentUpdate` and owns version-1 construction, validation, persistence-before-publication, Zellij transport, pruning, and snapshot delegation; `hooks/common/status_store.py` owns durable ordering. The [installation and extension contract](development.md#common-agent-adapter-interface) keeps both modules colocated with each adapter.
- **Codex hooks:** `hooks/codex/agent_status.py` maps lifecycle hooks and coordinates one locked exit watcher per Codex PID; `agent_notify.py` maps `agent-turn-complete` and can forward an existing notifier. Both dispatch through the common bridge.
- **Claude Code hooks:** `hooks/claude/agent_status.py` maps prompt, tool, permission, stop, and session-end hooks, uses prompt identity for terminal-completion ordering, and retains native terminal-sequence output while dispatching lifecycle updates through the common bridge.
- **OpenSpec:** baseline requirements live under `openspec/specs/`. The Claude adapter rationale and completion evidence are archived at `openspec/changes/archive/2026-07-21-add-claude-code-agent-status/`, and the accepted native-list rationale remains at `openspec/changes/archive/2026-07-19-refresh-zellij-native-sidebar-ui/`.
- **Theme/font:** Zellij's native nested-list renderer resolves selected/unselected list styling, while semantic item ranges color badges; Nerd Font Mono supplies single-cell status glyph metrics.

When changing hierarchy, preserve one `SidebarRow` source for render and input. When changing status, test protocol validation, stale ordering, pane reuse, tombstones, peer snapshots, leader turnover, watcher deduplication, cache migration, and cardinality-based placement. When changing formatting, cover narrow widths, wide characters, badge preservation, pane indentation, and the right inset. The concrete checks and scaling expectations live in [the testing strategy](development.md#testing-strategy) and [performance invariants](development.md#agent-status-performance-invariants).
