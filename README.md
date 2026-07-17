# zellij-vertical-tab

A [Zellij](https://zellij.dev) plugin that renders the session's tabs **vertically** in a mouse-resizable side pane, replacing the horizontal tab-bar. Multi-pane tabs include indented pane rows so agent state remains attached to the pane that owns it.

```
 1 editor           ┐
 2 services          │  one-pane tabs stay compact
    api             │  multi-pane tabs show children
    database        │  with per-pane agent status
▲3 very-long-tab-name…┘
```

## Features

- One compact row for tabs with zero or one terminal pane, prefixed with its index (matches `GoToTab` numbers)
- Indented, always-visible terminal-pane rows under tabs containing multiple panes; no expand/collapse control
- Rows keep a one-cell right edge inset; long tab and pane names end with a cell-aware `…` ellipsis instead of being cut off silently
- Active tab highlighted using your zellij theme
- Left-click a tab row to switch tabs or a pane row to focus that exact pane
- Scroll wheel moves the flattened tab-and-pane list when it overflows the pane height; `▲`/`▼` markers indicate hidden rows
- The active tab is always kept in view (follows keyboard tab switching)
- Codex lifecycle status shown as a right-aligned badge, vertically aligned and theme-colored: dim `` idle, cyan `` working, orange `` waiting for permission, or green `` answer ready
- Multiple panes in one tab show their Codex state independently on each pane row without an agent-name prefix

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

On first load zellij asks for the plugin's permissions (`ReadApplicationState` to see tabs, `ChangeApplicationState` to switch them, `ReadCliPipes` to receive agent status, and `MessageAndLaunchOtherPlugins` to synchronize the sidebar instances created for separate tabs). Approve with `y` — the choice is cached afterwards.

## Codex agent status

The repository includes a dependency-free Python bridge and user-level hook template under `hooks/codex/`. Install them once so Codex sessions launched from any project publish status:

```sh
mkdir -p ~/.codex/hooks
install -m 755 hooks/codex/agent_status.py ~/.codex/hooks/agent_status.py
install -m 755 hooks/codex/agent_notify.py ~/.codex/hooks/agent_notify.py
install -m 644 hooks/codex/hooks.json ~/.codex/hooks.json
```

If `~/.codex/hooks.json` already exists, merge the entries instead of replacing it. Also configure the external completion notifier in `~/.codex/config.toml` (replace `/Users/you` with your absolute home path):

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py"]
```

If you already have a notifier, preserve it by forwarding the original command and arguments before the final `--`:

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py", "--forward", "/path/to/existing-notifier", "existing-arg", "--"]
```

Codex runs the lifecycle bridge at session, prompt, pre-tool, permission, and stop boundaries. The external notifier covers completion paths such as code review that can omit `Stop`. Inside Zellij, both bridges publish a small versioned JSON message to the plugin; outside Zellij they exit successfully without changing status. Because Codex has no session-exit hook, the session-start handler also launches a detached watcher that clears the badge when that Codex process exits.

1. Start Codex from this repository inside a Zellij terminal pane.
2. Open `/hooks` in Codex and trust the user hook when prompted.
3. Submit a prompt. A one-pane tab, or the owning pane row in a multi-pane tab, will move through these states:

   | Badge | State |
   | --- | --- |
   | `` | Session started and is idle |
   | `` | Codex is working |
   | `` | Codex is waiting for permission |
   | `` | Codex has delivered an answer |

Status is tracked per terminal pane. A tab with one terminal pane keeps the badge on its compact tab row. A tab with multiple terminal panes shows all pane titles beneath the tab and puts each badge on its owning pane row; the parent tab has no duplicate aggregate badge. Tiled panes are ordered by screen position, followed by floating and suppressed panes.

Badge colors come from the active Zellij theme: idle is dimmed, working uses cyan emphasis, waiting uses orange emphasis, and done uses the success color. Selected tabs and the focused pane child retain full-row selected styling.

Closing a pane or exiting Codex clears its status, and starting a new Codex session in a reused pane replaces the old session. Codex initializes lifecycle hooks lazily, so a newly opened TUI may not show `` until its first prompt is submitted. Exit cleanup is best-effort if the bridge cannot identify the Codex ancestor process; closing the pane still clears the record.

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
- Rendering uses `Text`/`print_text_with_coordinates` (`ztext` APC sequences) so colors always follow the user's theme; active tab and focused pane rows are `.selected()` and padded to full width.
- `PaneUpdate` associates terminal panes with tabs and supplies pane titles, focus, layers, and geometry. The plugin flattens tabs and multi-pane children into the same row model used by rendering, scrolling, and mouse input.
- The `vertical-tab-agent-status` Zellij pipe carries versioned lifecycle messages from the user-level Codex hook. The plugin keeps only the newest session record per terminal pane and places it on the compact tab row or exact pane child as appropriate.

## License

MIT
