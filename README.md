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
- A Nerd Font for agent-status icons; the tested profile uses
  `0xProto Nerd Font Mono`

Download and verify the latest release:

```sh
plugin_dir="${ZELLIJ_PLUGIN_DIR:-$HOME/.config/zellij/plugins}"
mkdir -p "$plugin_dir"
curl -fL \
  https://github.com/yxwyoyoyo/zellij-vertical-tab/releases/latest/download/zellij_vertical_tab.wasm \
  -o "$plugin_dir/zellij_vertical_tab.wasm"
curl -fL \
  https://github.com/yxwyoyoyo/zellij-vertical-tab/releases/latest/download/zellij_vertical_tab.wasm.sha256 \
  -o "$plugin_dir/zellij_vertical_tab.wasm.sha256"
(cd "$plugin_dir" && shasum -a 256 -c zellij_vertical_tab.wasm.sha256)
```

To build from source instead, install [mise](https://mise.jdx.dev/) and run:

```sh
mise trust
mise install
mise run setup     # first checkout only
mise run install
```

Each [GitHub Release](https://github.com/yxwyoyoyo/zellij-vertical-tab/releases)
contains the optimized WASM and its SHA-256 checksum. Release assets are built
for the Zellij version stated in their notes; plugin ABI versions must match.

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

Download and verify the version-matched hook bundle once:

```sh
hooks_dir="${XDG_CACHE_HOME:-$HOME/.cache}/zellij-vertical-tab"
mkdir -p "$hooks_dir"
curl -fL \
  https://github.com/yxwyoyoyo/zellij-vertical-tab/releases/latest/download/agent-hooks.tar.gz \
  -o "$hooks_dir/agent-hooks.tar.gz"
curl -fL \
  https://github.com/yxwyoyoyo/zellij-vertical-tab/releases/latest/download/agent-hooks.tar.gz.sha256 \
  -o "$hooks_dir/agent-hooks.tar.gz.sha256"
(cd "$hooks_dir" && shasum -a 256 -c agent-hooks.tar.gz.sha256)
tar -xzf "$hooks_dir/agent-hooks.tar.gz" -C "$hooks_dir"
hooks_source="$hooks_dir/zellij-vertical-tab-hooks"
```

### Codex hooks

```sh
hooks_source="${XDG_CACHE_HOME:-$HOME/.cache}/zellij-vertical-tab/zellij-vertical-tab-hooks"
mkdir -p ~/.codex/hooks
install -m 755 "$hooks_source/codex/agent_status.py" ~/.codex/hooks/
install -m 755 "$hooks_source/codex/agent_notify.py" ~/.codex/hooks/
install -m 644 "$hooks_source/common/agent_bridge.py" ~/.codex/hooks/
install -m 644 "$hooks_source/common/status_store.py" ~/.codex/hooks/
```

If `~/.codex/hooks.json` does not exist, install
`$hooks_source/codex/hooks.json`; otherwise merge its `hooks` entries into the
existing file. Add this to `~/.codex/config.toml`, using your absolute home
path:

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py"]

[tui]
notifications = ["agent-turn-complete", "approval-requested"]
notification_method = "bel"
notification_condition = "always"
```

Start a new Codex session, open `/hooks`, and trust the user hook.

### Claude Code hooks

```sh
hooks_source="${XDG_CACHE_HOME:-$HOME/.cache}/zellij-vertical-tab/zellij-vertical-tab-hooks"
claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
mkdir -p "$claude_dir/hooks"
install -m 755 "$hooks_source/claude/agent_status.py" "$claude_dir/hooks/"
install -m 644 "$hooks_source/common/agent_bridge.py" "$claude_dir/hooks/"
install -m 644 "$hooks_source/common/status_store.py" "$claude_dir/hooks/"
```

Merge the `hooks` object from `$hooks_source/claude/settings.json` into
`$claude_dir/settings.json`; preserve every unrelated setting and existing hook.
Then start a new Claude Code session.

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
