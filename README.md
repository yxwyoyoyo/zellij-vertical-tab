# zellij-vertical-tab

A [Zellij](https://zellij.dev) plugin that replaces the horizontal tab bar with
a mouse-resizable vertical sidebar. Tabs with multiple panes expand into native
nested rows, keeping Codex and Claude Code status attached to the pane that owns
it.

```text
> editor                 
> services
  - api                  
  - database             
> very-long-tab-name…
```

## Highlights

- Zellij-native tab and pane hierarchy with full-width active-row styling
- Compact one-line display for tabs containing zero or one terminal pane
- Per-pane Codex and Claude Code lifecycle badges, without agent-name prefixes
- Native bell attention when an answer or approval request is waiting elsewhere
- Cell-aware ellipsis, vertical scrolling, mouse navigation, and boundary resize
- Status recovery across detach, reattach, session switching, and plugin reload

## Quick start

### Requirements

- Zellij **0.44.3** (`zellij-tile` must match the Zellij binary)
- [mise](https://mise.jdx.dev/) with the project tools installed
- A Nerd Font for agent-status icons; the tested profile uses
  `0xProto Nerd Font Mono`

Build and install the plugin:

```sh
mise trust
mise install
mise run setup     # first checkout only
mise run install
```

Add the sidebar to `~/.config/zellij/layouts/default.kdl`:

```kdl
layout {
    default_tab_template {
        pane split_direction="vertical" {
            pane size=32 borderless=true {
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

Start a fresh Zellij session and approve the plugin permissions when prompted.
New tabs inherit the sidebar. Drag the tiled boundary beside it to resize the
current tab's sidebar; pane frames are optional.

> [!IMPORTANT]
> Keep `children` wrapped in `pane { ... }`. On Zellij 0.44, placing it directly
> beside an unselectable plugin pane can crash the session.

The template removes Zellij's horizontal `zellij:tab-bar`. Add it back if you
want both tab displays.

## Agent status

Agent integration is optional. The sidebar uses the same pane-scoped states for
Codex and Claude Code:

| Badge | State |
| --- | --- |
| `` | Agent session is idle |
| `` | Agent is working |
| `` | Agent is waiting for permission |
| `` | An answer is ready |
| `` | Zellij retained attention for the tab |

Install the bridge for each agent you use. See
[Agent status integration](docs/agent-status.md) for Codex and Claude Code setup,
notification configuration, lifecycle behavior, recovery, and the common
adapter interface.

Status is tracked per terminal pane. One-pane tabs keep the badge on the compact
tab row; multi-pane tabs put it on the exact pane child. Returning to a pane with
a ready answer acknowledges that result and presents it as idle, while a newer
lifecycle event always takes precedence.

## Usage

- Click a tab row to switch tabs.
- Click a pane row to focus that pane.
- Scroll when the list overflows; `▲` and `▼` indicate hidden rows above or below.
- Drag the boundary between sidebar and content to resize it for the current tab.

Long names end with a terminal-cell-aware `…`; badges and the one-cell right
inset remain visible. Colors come from the active Zellij theme.

## Development

```sh
mise run test      # Rust and Python bridge tests
mise run dev       # build and launch the development layout
mise run reload    # rebuild and hot-reload inside Zellij
mise run check     # complete pre-PR gate
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for the daily workflow, OpenSpec process,
live verification, release steps, and adapter contract.

## Documentation

- [Agent status integration](docs/agent-status.md)
- [OpenWiki quickstart](openwiki/quickstart.md)
- [Architecture](openwiki/architecture.md)
- [Development workflow](DEVELOPMENT.md)

Generated OpenWiki pages are refreshed from the repository sources; edit the
source code and maintained documentation rather than those generated pages.

## License

MIT
