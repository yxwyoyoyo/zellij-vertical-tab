# zellij-vertical-tab

A [Zellij](https://zellij.dev) plugin that renders the session's tabs **vertically** — one row per tab — in a fixed-width side pane, replacing the horizontal tab-bar.

```
 1 Tab #1            ┐
 2 editor          ● │  24-column side pane
 3 tests           ?2│  (active row highlighted
▲12 long-tab-name  ○ ┘   with your theme)
```

## Features

- One row per tab, prefixed with its index (matches `GoToTab` numbers)
- Active tab highlighted using your zellij theme
- Left-click a row to switch to that tab
- Scroll wheel moves the list when tabs overflow the pane height; `▲`/`▼` markers indicate hidden rows
- The active tab is always kept in view (follows keyboard tab switching)
- Codex lifecycle status shown as a right-aligned badge: `○` idle, `●` working, `?` waiting for permission, or `✓` answer ready
- Multiple Codex panes in one tab are aggregated without an agent-name prefix (for example, `?2`)

## Requirements

- zellij **0.44.3** (the `zellij-tile` crate version must match the zellij binary — see `Cargo.toml`)
- Rust with the `wasm32-wasip1` target

## Build

```sh
rustup target add wasm32-wasip1        # once
cargo build --release --target wasm32-wasip1
# -> target/wasm32-wasip1/release/zellij_vertical_tab.wasm
```

## Try it

```sh
cargo build --target wasm32-wasip1     # debug build
zellij -l zellij.kdl                   # from this directory
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
3. Submit a prompt. The owning tab will move through these states:

   | Badge | State |
   | --- | --- |
   | `○` | Session started and is idle |
   | `●` | Codex is working |
   | `?` | Codex is waiting for permission |
   | `✓` | Codex has delivered an answer |

Status is tracked per terminal pane. If a tab contains multiple Codex panes, the badge appends the total pane count and uses `waiting`, then `working`, then `done`, then `idle` precedence. For example, one waiting pane and two working panes render as `?3`.

Closing a pane or exiting Codex clears its status, and starting a new Codex session in a reused pane replaces the old session. Codex initializes lifecycle hooks lazily, so a newly opened TUI may not show `○` until its first prompt is submitted. Exit cleanup is best-effort if the bridge cannot identify the Codex ancestor process; closing the pane still clears the record.

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
               pane size=24 borderless=true {
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

## Development

```sh
cargo test                             # unit tests (host target)
zellij action start-or-reload-plugin file:target/wasm32-wasip1/debug/zellij_vertical_tab.wasm
```

## How it works

- The plugin is a **bin crate**: zellij requires the wasm module to export `_start` (command-style module), which only bin targets provide. `register_plugin!` generates `main()`.
- `set_selectable(false)` makes the pane unfocusable (same pattern as the built-in tab-bar) and is what makes a fixed `size=24` pane stable. On zellij 0.44 it must not be called during initial startup when the pane lives in a `default_tab_template`, so the plugin defers it to the first event. Unselectable panes still receive mouse events.
- Rendering uses `Text`/`print_text_with_coordinates` (`ztext` APC sequences) so colors always follow the user's theme; the active row is `.selected()` and padded to full width.
- `PaneUpdate` associates terminal panes with tabs, while the `vertical-tab-agent-status` Zellij pipe carries versioned lifecycle messages from the user-level Codex hook. The plugin keeps only the newest session record per terminal pane and aggregates records at render time.

## License

MIT
