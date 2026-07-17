---
type: Engineering Runbook
title: Development, Testing, and Operations
description: Mise-managed runbook for building, testing, releasing, installing, hot-reloading, fresh-session layout verification, and troubleshooting the pane-aware zellij-vertical-tab plugin and its Codex status bridge.
resource: AGENTS.md
tags: [development, testing, operations, mise, zellij, codex]
---

# Development, testing, and operations

This runbook applies the invariants in the [plugin architecture](architecture.md). For product behavior and repository navigation, start at the [quickstart](quickstart.md).

## Mise-managed workflows

`AGENTS.md` and `DEVELOPMENT.md` are the canonical maintainer workflow. The checked-in `mise.toml` pins Rust 1.97.1 and Node 26.5.0 and exposes task aliases so tests, builds, runtime helpers, releases, specifications, and documentation use consistent entrypoints.

### Bootstrap and task entrypoints

```sh
mise trust
mise install
mise run setup
```

`setup` adds `wasm32-wasip1` and installs the pinned OpenSpec 1.6.0 and OpenWiki 0.2.0 CLIs. The normal loop is:

```sh
mise run test  # cargo test + Python Codex bridge tests
mise run build # debug WASM
mise run dev   # build, then launch zellij.kdl
mise run check # complete pre-PR gate
```

The host Rust tests exercise pure status, pane hierarchy, row target, viewport, cell-width, inset, and styling helpers. The Python tests exercise lifecycle mapping, process-exit cleanup, completion notification parsing, and notifier forwarding. The debug artifact is `target/wasm32-wasip1/debug/zellij_vertical_tab.wasm`.

### Development launch, reload, and status restoration

`mise run dev` must start from the repository root because `zellij.kdl` loads the relative debug artifact. After an edit, `mise run reload` first rebuilds and then delegates to `scripts/reload-plugin debug`:

```sh
mise run reload                       # current Zellij session
mise run reload -- Hub                # named session from another terminal
ZELLIJ_DEV_SESSION=Hub mise run reload
```

The helper resolves the artifact to an absolute path, verifies it exists, and calls `start-or-reload-plugin`. It targets the explicit argument, then `ZELLIJ_DEV_SESSION`, then the current `ZELLIJ_SESSION_NAME`; outside Zellij it fails rather than guessing a session. `mise run deploy -- Hub` applies the same targeting rules to the installed release artifact after the release gate and installation.

Reloading replaces the plugin instance and therefore clears its in-memory, pane-keyed agent records. The plugin deliberately has no durable store and cannot safely infer every live Codex state. If the current pane, Codex session ID, and state are known, republish that exact state through the normal `vertical-tab-agent-status` protocol:

```sh
mise run status -- terminal_0 <codex-session-id> done Hub
```

`scripts/publish-agent-status` accepts only `terminal_<number>` pane IDs and `idle`, `working`, `waiting`, `done`, or `clear`; it emits a version-1 payload with a fresh timestamp and uses the same explicit/environment/current-session targeting order. Starting another prompt also republishes state through the normal Codex hooks. A stale WASM can make source-level debugging misleading (`tasks/lessons.md`), and startup-template or permission behavior still requires a fresh session rather than reload.

## Codex bridge installation

Install the dependency-free bridge and hook template once at user scope so Codex sessions from any project can publish into the owning Zellij terminal pane:

```sh
mkdir -p ~/.codex/hooks
install -m 755 hooks/codex/agent_status.py ~/.codex/hooks/agent_status.py
install -m 755 hooks/codex/agent_notify.py ~/.codex/hooks/agent_notify.py
install -m 644 hooks/codex/hooks.json ~/.codex/hooks.json
```

If `~/.codex/hooks.json` exists, merge the entries rather than overwriting it. Configure completion notification in `~/.codex/config.toml` with the absolute home path:

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py"]
```

To preserve an existing notifier, place its command and fixed arguments between `--forward` and the final `--`, as shown in `README.md`. The lifecycle bridge maps `SessionStart` to idle, `UserPromptSubmit`/`PreToolUse` to working, `PermissionRequest` to waiting, and `Stop` to done. The notification bridge covers `agent-turn-complete`; the detached watcher emits clear when the Codex process exits. Both bridges are best-effort and return success when outside Zellij or when publication fails.

## Testing strategy

### Rust host tests

Run `mise run test`; its Rust tests are colocated in `src/main.rs` and currently cover:

- viewport bounds and active-tab following over flattened hierarchy rows;
- terminal-pane filtering and deterministic tiled/floating/suppressed ordering;
- compact zero/one-pane tabs and expanded multi-pane children;
- focused tiled/floating child selection and empty-title fallback;
- exact tab versus pane row targets;
- payload validation, timestamps, pane reuse, clear tombstones, cleanup, peer discovery, and snapshots;
- cell-aware truncation, wide characters, ellipsis, index/pane alignment, badge preservation, one-cell inset, and theme styling.

Keep runtime host calls out of these tests unless a proper mock layer is introduced; the non-WASM import stub exists only to link pure tests.

### Python bridge tests

`mise run test` also runs `python3 -m unittest discover -s hooks/codex -p 'test_*.py'`. Add cases whenever lifecycle mapping, notification payload fields, forwarding syntax, or process cleanup changes.

### WASM and specification checks

Always pair host tests with a WASM build because host success does not prove the runtime target or `_start` module shape. `mise run check` is the complete pre-PR gate: formatting, Rust and Python tests, Clippy with warnings denied, debug WASM, strict validation of all OpenSpec artifacts, and `git diff --check`. `mise run release` depends on that gate before building release WASM. The archived pane-aware completion record (`openspec/changes/archive/2026-07-17-nest-panes-under-tabs/tasks.md`) provides additional live/headless verification evidence.

The merged behavior contract is `openspec/specs/`, and completed proposals and rationale are under `openspec/changes/archive/`. The accepted flexible-layout and native-boundary-drag change, including its disposable-session verification and completed local gate, is archived at `openspec/changes/archive/2026-07-18-add-mouse-resizable-sidebar/`.

## Runtime verification

Lifecycle, permissions, peer synchronization, layout safety, theme styling, and mouse focus require Zellij. The macOS PTY harness in `AGENTS.md` is:

```sh
( ( sleep 10 ) | env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
    TERM=xterm-256color script -q /tmp/e2e.log zellij -l zellij.kdl ) &
sleep 8 && zellij list-sessions
zellij kill-session <name>
```

Unset the Zellij variables so the process does not think it is nested. The input pipe may approve permissions with `y` and inject SGR mouse sequences. A minimum pane-aware pass verifies:

1. startup survives permission grant and renders a sidebar initially sized to 13% with no horizontal tab bar;
2. with normal Zellij mouse handling, dragging the sidebar/content boundary changes the current tab's sidebar width even when pane frames are hidden; a new tab starts at 13% rather than inheriting that width;
3. one-pane tabs stay compact, while a multi-pane tab lists all terminal panes and excludes plugin panes;
4. pane children follow tiled/floating/suppressed visual order and the focused visible-layer child is selected;
5. one-pane status appears on the tab, multi-pane statuses remain on exact children, and no aggregate/count appears;
6. a new tab's sidebar obtains current statuses from an existing peer;
7. a tab-row click switches tabs, while a pane-row click focuses that exact pane, including after scrolling;
8. long ASCII/wide names ellipsize, badges remain intact, and every normal-width row keeps its rightmost cell blank;
9. wheel overflow and keyboard tab switching operate on the flattened hierarchy.

## Runbook

### Blank sidebar or missing status

- Confirm all four plugin permissions were granted. `ReadApplicationState` gates tab/pane updates; `ReadCliPipes` gates Codex input; peer synchronization needs `MessageAndLaunchOtherPlugins`.
- Confirm the KDL path points to a freshly built artifact and that `zellij-tile` matches Zellij 0.44.3.
- For a missing Codex badge, confirm the global hooks are installed/merged and the process has `ZELLIJ_PANE_ID`. A newly launched Codex TUI may not emit idle until hooks initialize on the first prompt.
- If only a newly created tab lacks existing statuses, inspect peer discovery/snapshot handling rather than reintroducing tab-level aggregation.

### Wrong row or pane activates

Trace the click through `build_sidebar_rows` and `RowTarget`. Mouse lines and `scroll_offset` address the flattened hierarchy; tab targets use one-based switching, while pane targets use stable terminal IDs. Never compute render and click rows independently.

### Status appears on the wrong row

Check pane cardinality first. Exactly one terminal pane owns the parent badge; more than one moves all statuses to children and leaves the parent empty. Plugin panes do not count. Then inspect `pane_tabs`, `terminal_panes`, and the pane-keyed record rather than tab names or titles.

### Row touches the right edge or clips a badge

Keep `ROW_RIGHT_PADDING = 1`, reserve badge width before fitting the body, and exclude the trailing cell from the badge color range. Use terminal-cell width rather than Rust character count, and extend narrow/wide-character tests with the change.

### Sidebar does not resize

Confirm the layout uses flexible `size="13%"`, not fixed `size=32`, and that Zellij mouse handling is enabled. Drag the one-cell **boundary between the sidebar and content**. Pane frames may be enabled to make that boundary visible, but neither pane frames nor advanced mouse actions are required for resizing. Start a fresh session after changing layout or configuration: hot reload replaces plugin code but does not reconstruct existing pane geometry. Width changes are tab-local and intentionally not persisted.

### Client exits at startup

Keep `set_selectable(false)` deferred until the first event and keep `children` wrapped in `pane { ... }`. Test template changes in a fresh Zellij session, not only via hot reload. On macOS, inspect the Zellij log path described in `tasks/lessons.md` and vary one lifecycle/layout condition at a time.

### “Could not find exported function”

Confirm the explicit binary target and `register_plugin!(State)` remain. A reactor/cdylib build does not expose the `_start` entrypoint expected by Zellij.

## Release and maintenance notes

- Treat a Zellij upgrade as an ABI migration: update the binary, `zellij-tile`, lockfile, and runtime test environment together.
- `mise run release` gates and builds release WASM; `mise run install` copies it to `${ZELLIJ_PLUGIN_DIR:-$HOME/.config/zellij/plugins}`, and `mise run deploy -- <session>` installs then reloads it. Startup-sensitive changes still need a new session.
- No checked-in product CI publishes WASM; release and installation remain maintainer-run tasks.
- Keep baseline OpenSpec files synchronized with implemented behavior, then archive completed changes with their proposal/design/task evidence. `mise run spec` performs strict validation.
- Run `mise run docs` only after code and specifications stabilize; it invokes an OpenWiki code-mode update. `.github/workflows/openwiki-update.yml` also updates documentation daily or on demand, but does not build, test, install, or release the plugin.
