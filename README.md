# zellij-vertical-tab

A [Zellij](https://zellij.dev) plugin that renders the session's tabs **vertically** — one row per tab — in a fixed-width side pane, replacing the horizontal tab-bar.

```
 1 Tab #1          ┐
 2 editor          │  20-column side pane
 3 tests           │  (active row highlighted
▲12 long-tab-name  ┘   with your theme)
```

## Features

- One row per tab, prefixed with its index (matches `GoToTab` numbers)
- Active tab highlighted using your zellij theme
- Left-click a row to switch to that tab
- Scroll wheel moves the list when tabs overflow the pane height; `▲`/`▼` markers indicate hidden rows
- The active tab is always kept in view (follows keyboard tab switching)

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

On first load zellij asks for the plugin's permissions (`ReadApplicationState` to see tabs, `ChangeApplicationState` to switch them). Approve with `y` — the choice is cached afterwards.

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
               pane size=20 borderless=true {
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
- `set_selectable(false)` makes the pane unfocusable (same pattern as the built-in tab-bar) and is what makes a fixed `size=20` pane stable. On zellij 0.44 it must not be called during initial startup when the pane lives in a `default_tab_template`, so the plugin defers it to the first event. Unselectable panes still receive mouse events.
- Rendering uses `Text`/`print_text_with_coordinates` (`ztext` APC sequences) so colors always follow the user's theme; the active row is `.selected()` and padded to full width.

## License

MIT
