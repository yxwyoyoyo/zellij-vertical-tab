---
type: Architecture Guide
title: Plugin Architecture and Domain Model
description: Runtime architecture of the pane-aware Zellij sidebar, covering native nested-list rendering, terminal-pane Codex status ownership, peer synchronization, durable same-server recovery, input, and Zellij-owned mouse-resizable layout constraints.
resource: src/main.rs
tags: [architecture, zellij, rust, wasm, codex, panes]
---

# Plugin architecture and domain model

The plugin is one Rust binary compiled to `wasm32-wasip1`. Zellij owns tabs, panes, focus, dimensions, theme resolution, native bell state, and application mutations; the plugin retains presentation state, builds a deterministic hierarchy, and maps visible rows back to Zellij targets. The [quickstart](quickstart.md) gives product and source orientation, while [development and operations](development.md) turns these constraints into repeatable workflows.

## Runtime boundary and lifecycle

```text
TabUpdate ───────────────> tabs / active tab
PaneUpdate ──────────────> pane ownership + ordered terminal metadata + peers
Codex hooks ──host journal + Zellij pipe> pane-keyed AgentRecord ──peer messages──> sidebar instances
             plugin /cache snapshots <──── runtime restart ────────┘
Codex TUI ──BEL> Zellij visual bell ──TabInfo.has_bell_notification──> tab attention icon
                                      │
                                      v
                         flattened Vec<SidebarRow>
                         │        │         │
                         render   scroll    click target
```

`load()` records this sidebar's plugin ID and Zellij server PID; restores matching snapshots from `/cache`; requests `ReadApplicationState`, `ChangeApplicationState`, `ReadCliPipes`, `MessageAndLaunchOtherPlugins`, and `RunCommands`; and subscribes to tab, pane, mouse, permission, and command-result events. Once permission is granted, the instance invokes a fixed global helper once to reconcile host-journal events emitted while detached. The first `update()` calls `set_selectable(false)` once. `render()` converts the visible hierarchy into `NestedListItem` values and emits one native list with `print_nested_list_with_coordinates`; `pipe()` validates external Codex updates and handles instance-to-instance synchronization (`src/main.rs`).

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
| `plugin_id`, `zellij_pid`, `peer_plugin_ids` | Identity of this sidebar, its live Zellij server, and same-URL sidebar instances in other tabs |
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

## Per-pane Codex status and synchronization

The [Codex bridge workflow](development.md#codex-bridge-installation) publishes version-1 JSON on `vertical-tab-agent-status` with pane ID, session ID, state, millisecond timestamp, and optional normalized lifecycle event and Codex turn ID. States are `idle`, `working`, `waiting`, `done`, and `clear`; their visible glyphs use dim, cyan emphasis, orange emphasis, and success styling respectively. A manually reviewed `PermissionRequest` enters waiting, while a request whose turn context identifies automatic review remains working. `PostToolUse` returns either path to working after an approved tool finishes. Within the same turn, done is terminal against delayed tool and permission events; a new `UserPromptSubmit` reopens the session as working. Legacy version-1 records without optional lifecycle metadata remain valid.

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

There is no dominant-state precedence and no pane-count suffix in the current design. This preserves exact ownership when two Codex sessions in one tab are in different states. Native attention is separately tab-scoped: a one-pane compact row can show `status bell`, while a multi-pane parent shows the bell and its children retain their own status icons.

### Sidebar instance convergence

`default_tab_template` creates a separate plugin instance in every tab, so one instance receiving a CLI pipe or focus event is not enough. On `PaneUpdate`, an instance discovers plugin panes with the same plugin URL. A newly discovered peer receives a sync request and the sender's current focus baseline; the peer returns a serialized snapshot containing lifecycle records and acknowledgement references; timestamp validation merges only current state. Normal external updates are forwarded once as sync updates. Changed focus observations are sent once to peers, which update their baseline without forwarding. A newly created focus acknowledgement uses its own validated pane/session/timestamp message; recipients apply it without forwarding, so there is no loop. An acknowledgement may arrive before its matching status and becomes visible only after that exact `done` record arrives.

### Durable recovery

The Python bridges use `status_store.py` to write each validated lifecycle event before pipe publication. Records are isolated by Zellij server PID and terminal pane, serialized under an advisory per-pane lock, ordered by timestamp, session, turn, and terminal-completion rules, and replaced atomically. This host journal remains available when a detached session has no plugin runtime to receive an undirected pipe, including a watcher-generated `clear` after Codex exits.

Every plugin mutation also serializes lifecycle records and exact acknowledgement references to a per-server, per-plugin file under `/cache`. On `load()`, an instance scans only bounded, well-formed files for its current server PID and merges them through the normal timestamp, session, turn, and terminal-completion rules. It deliberately does not restore `focused_terminal_panes`, so runtime startup cannot fabricate a focus transition or acknowledge an unseen completion.

After `RunCommands` is granted, the plugin runs the fixed global `agent_status.py --snapshot <zellij-pid>` helper once. Only a successful, bounded UTF-8 result with the matching command context is parsed. Host recovery, cache recovery, peer snapshots, and live pipe updates all enter through the same versioned snapshot/status validators and turn-aware ordering, so arrival path cannot change the resulting lifecycle state. Invalid files, denied permission, helper failure, and unavailable persistence are ignored without disabling in-memory status handling. Server-PID namespacing prevents another live Zellij server with reused pane IDs from inheriting state; persistence is not intended to resurrect a session after its server process exits.

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
- **Codex hooks:** `hooks/codex/agent_status.py` maps lifecycle hooks, starts best-effort exit cleanup, and serves recovery snapshots; `agent_notify.py` covers `agent-turn-complete` paths and can forward an existing notifier; both journal valid events through `status_store.py` before pipe publication.
- **OpenSpec:** current requirements live under `openspec/specs/`; the accepted native-list rationale and completion evidence are archived at `openspec/changes/archive/2026-07-19-refresh-zellij-native-sidebar-ui/`.
- **Theme/font:** Zellij's native nested-list renderer resolves selected/unselected list styling, while semantic item ranges color badges; Nerd Font Mono supplies single-cell status glyph metrics.

When changing hierarchy, preserve one `SidebarRow` source for render and input. When changing status, test protocol validation, stale ordering, pane reuse, tombstones, peer snapshots, and cardinality-based placement. When changing formatting, cover narrow widths, wide characters, badge preservation, pane indentation, and the right inset. The concrete checks live in [the testing strategy](development.md#testing-strategy).
