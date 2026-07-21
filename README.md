# zellij-vertical-tab

A [Zellij](https://zellij.dev) plugin that renders the session's tabs **vertically** in a mouse-resizable side pane, replacing the horizontal tab-bar. Multi-pane tabs include indented pane rows so agent state remains attached to the pane that owns it.

```
 > editor                 ┐
 > services                │  one-pane tabs stay compact
   - api                 │  multi-pane tabs use Zellij's
   - database            │  native nested-list hierarchy
 > ▲very-long-tab-name…  ┘
```

## Features

- One compact row for tabs with zero or one terminal pane; the native `>` bulletin is the sole leading marker, without a redundant displayed tab number
- Zellij-native `>` tab bullets and indented `-` terminal-pane bullets, styled from the active theme; no expand/collapse control
- Rows keep a one-cell right edge inset; long tab and pane names end with a cell-aware `…` ellipsis instead of being cut off silently
- Active tabs and focused pane rows use Zellij's full-width selected-list styling
- Left-click a tab row to switch tabs or a pane row to focus that exact pane
- Scroll wheel moves the flattened tab-and-pane list when it overflows the pane height; `▲`/`▼` markers indicate hidden rows
- The active tab is always kept in view (follows keyboard tab switching)
- Codex and Claude Code lifecycle status shown as a right-aligned badge, vertically aligned and theme-colored: dim `` idle, cyan `` working, orange `` waiting for permission, or green `` answer ready
- Multiple panes in one tab show their agent state independently on each pane row without an agent-name prefix
- Codex and Claude Code answer-ready and approval events use Zellij's native visual bell and retain an orange `` attention icon on an inactive owning tab until Zellij acknowledges it
- Returning to a pane whose current agent state is `done` acknowledges that exact result and presents it as idle without rewriting the lifecycle record
- Agent lifecycle and acknowledgement state recovers across client detach, session switching, reattach, and plugin hot reload while the same Zellij server remains alive

## Requirements

- zellij **0.44.3** (the `zellij-tile` crate version must match the zellij binary — see `Cargo.toml`)
- Rust with the `wasm32-wasip1` target
- A Nerd Font for the agent-status icons (the tested iTerm profile uses `0xProto Nerd Font Mono`)

## Build

```sh
mise trust
mise install
mise run setup                         # once
mise run release
# -> target/wasm32-wasip1/release/zellij_vertical_tab.wasm
```

## Try it

```sh
mise run dev
```

On first load zellij asks for the plugin's permissions (`ReadApplicationState` to see tabs, `ChangeApplicationState` to switch them, `ReadCliPipes` to receive agent status, `MessageAndLaunchOtherPlugins` to synchronize the sidebar instances created for separate tabs, and `RunCommands` to recover lifecycle events emitted while detached). Approve with `y` — the choice is cached afterwards.

## Agent status

The sidebar supports Codex and Claude Code through the same agent-neutral,
pane-scoped protocol. Install the bridge for each agent you use.

### Codex

The repository includes a dependency-free Python bridge and user-level hook template under `hooks/codex/`. Install them once so Codex sessions launched from any project publish status:

```sh
mkdir -p ~/.codex/hooks
install -m 755 hooks/codex/agent_status.py ~/.codex/hooks/agent_status.py
install -m 755 hooks/codex/agent_notify.py ~/.codex/hooks/agent_notify.py
install -m 644 hooks/codex/status_store.py ~/.codex/hooks/status_store.py
install -m 644 hooks/codex/hooks.json ~/.codex/hooks.json
```

If `~/.codex/hooks.json` already exists, merge the entries instead of replacing it. Also configure the external completion notifier in `~/.codex/config.toml` (replace `/Users/you` with your absolute home path):

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py"]

[tui]
notifications = ["agent-turn-complete", "approval-requested"]
notification_method = "bel"
notification_condition = "always"
```

If you already have a notifier, preserve it by forwarding the original command and arguments before the final `--`:

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py", "--forward", "/path/to/existing-notifier", "existing-arg", "--"]
```

Codex runs the lifecycle bridge at session, prompt, pre-tool, permission, post-tool, and stop boundaries. A manually reviewed `PermissionRequest` publishes `waiting`. For an auto-reviewed turn, the bridge keeps `working` by reading the optional reviewer identity from Codex's transcript; if that context is absent or unreadable, it conservatively publishes `waiting`. `PostToolUse` also publishes `working` when an approved tool finishes and control returns to the agent. The external notifier covers completion paths such as code review that can omit `Stop`. Inside Zellij, both bridges first write a server- and pane-scoped host journal, then publish a small versioned JSON message to the plugin. Because Codex has no session-exit hook, the session-start handler also launches a detached watcher that journals and publishes `clear` when that Codex process exits. Outside Zellij, the bridges exit successfully without changing status.

Each status record may include its normalized lifecycle event and Codex turn ID. Within a turn, `done` is terminal: a delayed pre-tool, permission, or post-tool event cannot reopen it, while `UserPromptSubmit` starts the next turn as `working`. Legacy version-1 records without this optional metadata remain readable and use the same conservative completion boundary.

Each sidebar instance also saves lifecycle records and exact focus acknowledgements in Zellij's plugin cache. When a sidebar runtime starts again, it restores that cache immediately and requests one host-journal snapshot to reconcile events that occurred while no client was attached. Newer applicable records win through the normal timestamp, session, turn, and terminal-completion rules, so detached `working`, `waiting`, `done`, and `clear` events replace stale cached state without overwriting a newer live update. Recovery is best-effort: denied `RunCommands` permission, a missing helper, or corrupt cache/journal data leaves normal live pipes and peer synchronization working. Existing sessions begin durable host journaling at their next lifecycle event. State is isolated by the live Zellij server process and does not resurrect after that server exits and a different server starts.

The TUI notification settings are independent of the external `notify` bridge: when a Codex turn completes or needs approval, Codex emits BEL. `always` is required because switching Zellij panes or tabs does not make Codex's terminal-focus detector report `unfocused` in every terminal setup. Zellij flashes an active tab or retains native bell state for an inactive tab, and the sidebar shows `` on a retained tab until Zellij clears it. Start a new Codex session after changing `config.toml`. Zellij exposes retained bell ownership per tab, so a multi-pane tab keeps the bell on its parent row while each pane child keeps its exact Codex status.

1. Start Codex from this repository inside a Zellij terminal pane.
2. Open `/hooks` in Codex and trust the user hook when prompted.
3. Submit a prompt. A one-pane tab, or the owning pane row in a multi-pane tab, will move through these states:

   | Badge | State |
   | --- | --- |
   | `` | Session started and is idle |
   | `` | Codex is working |
   | `` | Codex is waiting for permission |
   | `` | Codex has delivered an answer |
   | `` | Zellij has retained attention for this tab |

Status is tracked per terminal pane. A tab with one terminal pane keeps the badge on its compact tab row. A tab with multiple terminal panes shows all pane titles beneath the tab and puts each badge on its owning pane row; the parent tab has no duplicate aggregate badge. Tiled panes are ordered by screen position, followed by floating and suppressed panes.

Badge colors come from the active Zellij theme: idle is dimmed, working uses cyan emphasis, waiting and native bell attention use orange emphasis, and done uses the success color. Selected tabs and the focused pane child retain full-row selected styling.

Returning to a completed pane in a tab viewed by an attached Zellij client acknowledges its current `done` record and changes the visible badge from `` to idle ``. A completion that arrives while its pane remains focused stays `done` until the user leaves and returns. The plugin acknowledges only a confirmed focus transition, rather than status arrival against potentially stale tab metadata, so an unseen tab cannot turn idle during a tab-switch race. Because every tab owns a separate sidebar instance, changed client-viewed pane sets are shared between peers so leaving through one tab and returning through another forms one session-wide focus history. The acknowledgement is tied to that record's Codex session ID and timestamp, so a newer lifecycle event immediately replaces the idle presentation. `working` and `waiting` are never acknowledged by focus. Status acknowledgement remains separate from Zellij's native bell state and clearing behavior.

Closing a pane or exiting Codex clears its status, and starting a new Codex session in a reused pane replaces the old session. Codex initializes lifecycle hooks lazily, so a newly opened TUI may not show `` until its first prompt is submitted. Exit cleanup and detached recovery are best-effort if the bridge cannot identify the Zellij server ancestor; closing the pane still clears the plugin record after a client is attached.

### Claude Code

Install the Claude bridge and the same dependency-free durable store under the
user-level Claude configuration directory:

```sh
mkdir -p ~/.claude/hooks
install -m 755 hooks/claude/agent_status.py ~/.claude/hooks/agent_status.py
install -m 644 hooks/codex/status_store.py ~/.claude/hooks/status_store.py
```

Merge the `hooks` object from `hooks/claude/settings.json` into
`~/.claude/settings.json`. Preserve existing `env`, model, theme, permissions,
plugins, and hook handlers; do not replace the whole settings file. Start a new
Claude Code session after changing the file. If `CLAUDE_CONFIG_DIR` points
elsewhere, install both files there and adjust the merged hook command paths;
the plugin's recovery lookup honors that environment variable.

Claude publishes idle at session start; working when a prompt starts or tool
control returns to the agent; waiting when a permission dialog appears; done
when the response stops; and clear when the session ends or switches. Claude's
prompt ID is used as the turn identity, so a delayed tool event cannot reopen a
completed prompt while the next prompt can. `SessionEnd` handles normal exit,
`/clear`, and interactive session switching. An abrupt process failure in a
still-open pane remains best-effort until the pane closes or another agent
session starts there.

On Claude Code 2.1.141 or newer, the same bridge returns one supported terminal
BEL sequence when a visible permission request appears and when a final answer
stops. Zellij can then retain native attention on the owning tab and render the
same `` icon used for Codex. A continuing stop hook does not ring, and the
bridge emits no bell for ordinary working events. This does not approve, deny,
or block any Claude action. Native bell retention is tab-scoped in Zellij while
the lifecycle badge remains attached to the exact pane.

Codex and Claude Code can run concurrently in different panes. Both use the
same badges and colors, and the UI deliberately does not add an agent-name
prefix. Their records remain isolated by Zellij server, terminal pane, agent
session, prompt or turn identity, and timestamp. Either installed bridge can
serve the shared host-journal snapshot used after detach or plugin reload.

## Install for everyday use

1. Copy the plugin:

   ```sh
   mkdir -p ~/.config/zellij/plugins
   cp target/wasm32-wasip1/release/zellij_vertical_tab.wasm ~/.config/zellij/plugins/
   ```

2. Create (or edit) `~/.config/zellij/layouts/default.kdl`:

   ```kdl
   layout {
       default_tab_template {
           pane split_direction="vertical" {
               pane size="13%" borderless=true {
                   plugin location="file:~/.config/zellij/plugins/zellij_vertical_tab.wasm"
               }
               pane {
                   children
               }
           }
           pane size=1 borderless=true {
               plugin location="zellij:status-bar"
           }
       }
   }
   ```

   Notes:
   - `children` **must** stay wrapped in its own `pane { ... }`: with `children` as a direct sibling, an unselectable plugin pane crashes the session on zellij 0.44.
   - This removes the horizontal `zellij:tab-bar` entirely; keep it in the template if you want both.
   - New tabs (`Ctrl t n`) inherit the template, so the side pane appears everywhere.

3. Start a fresh Zellij session, then drag the tiled boundary between the
   sidebar and content to resize the sidebar. Zellij mouse handling is enabled
   by default. Pane frames are optional: `pane_frames true` makes the boundary
   visible as the content pane's left frame, while hidden frames leave the same
   one-cell drag target. The sidebar stays borderless and unfocusable. Width is
   local to each tab and is not persisted, so a new tab starts again at the
   layout's `13%` width.

## Development

```sh
mise run test    # fast Rust + bridge tests
mise run reload  # rebuild and hot-reload inside Zellij
mise run check   # complete pre-PR gate
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for the feature, OpenSpec, live
verification, status restoration, documentation, and release workflows.

## How it works

- The plugin is a **bin crate**: zellij requires the wasm module to export `_start` (command-style module), which only bin targets provide. `register_plugin!` generates `main()`.
- `set_selectable(false)` makes the pane unfocusable (same pattern as the built-in tab-bar). On zellij 0.44 it must not be called during initial startup when the pane lives in a `default_tab_template`, so the plugin defers it to the first event. The percentage layout keeps the pane flexible so Zellij's native boundary drag can resize it.
- Rendering uses Zellij's `NestedListItem` component, including its native hierarchy bullets, selected/unselected list colors, bold text, and full-width selection surface. Badge ranges layer their semantic theme colors onto the same list items.
- `PaneUpdate` associates terminal panes with tabs and supplies pane titles, focus, layers, and geometry. The plugin flattens tabs and multi-pane children into the same row model used by rendering, scrolling, and mouse input.
- `TabUpdate` supplies Zellij's persistent native bell state. The plugin displays that attention at tab scope and leaves exact agent lifecycle ownership on the appropriate compact tab or pane child row.
- The `vertical-tab-agent-status` Zellij pipe carries versioned lifecycle messages from the user-level Codex and Claude Code hooks. The plugin keeps only the newest session record per terminal pane and places it on the compact tab row or exact pane child as appropriate.
- Focus acknowledgement uses a separate internal peer message and snapshot field keyed to the exact completed record; it never fabricates an external agent `idle` event.

## License

MIT
