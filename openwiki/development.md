---
type: Engineering Runbook
title: Development, Testing, and Operations
description: Mise-managed runbook for building, testing, releasing, installing, hot-reloading, and troubleshooting the pane-aware zellij-vertical-tab plugin and its Codex status bridge.
resource: AGENTS.md
tags: [development, testing, operations, mise, zellij, codex]
---

# Development, testing, and operations

This runbook applies the invariants in the [plugin architecture](architecture.md). For product behavior and repository navigation, start at the [quickstart](quickstart.md).

## Mise-managed workflows

The maintainer machine has no system Cargo; Rust commands are run through `mise exec --` (`AGENTS.md`). The repository does not define checked-in mise task aliases, so use the explicit commands below. Add `wasm32-wasip1` once with `rustup target add wasm32-wasip1`.

### Build and test

```sh
mise exec -- cargo test
python3 -m unittest discover -s hooks/codex -p 'test_*.py'
mise exec -- cargo build --target wasm32-wasip1
```

The host Rust tests exercise pure status, pane hierarchy, row target, viewport, cell-width, inset, and styling helpers. The Python tests exercise lifecycle mapping, process-exit cleanup, completion notification parsing, and notifier forwarding. The debug artifact is `target/wasm32-wasip1/debug/zellij_vertical_tab.wasm`.

### Release build and install

```sh
mise exec -- cargo build --release --target wasm32-wasip1
mkdir -p ~/.config/zellij/plugins
install -m 644 target/wasm32-wasip1/release/zellij_vertical_tab.wasm \
  ~/.config/zellij/plugins/zellij_vertical_tab.wasm
```

The release profile enables LTO, size optimization, one codegen unit, and stripping (`Cargo.toml`). Configure `~/.config/zellij/layouts/default.kdl` from the README example: use `size=32`, keep `children` nested in its own pane, and point the plugin location at the installed WASM.

### Development launch and hot reload

```sh
mise exec -- cargo build --target wasm32-wasip1
zellij -l zellij.kdl
```

Run the layout from the repository root because it loads a relative debug artifact. For a running development session, rebuild and then reload:

```sh
mise exec -- cargo build --target wasm32-wasip1
zellij action start-or-reload-plugin \
  file:target/wasm32-wasip1/debug/zellij_vertical_tab.wasm
```

A stale WASM can make source-level debugging misleading (`tasks/lessons.md`). Hot reload is suitable for rendering and state iteration; startup-template and permission behavior still needs a fresh session.

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

Run `mise exec -- cargo test`. Tests are colocated in `src/main.rs` and currently cover:

- viewport bounds and active-tab following over flattened hierarchy rows;
- terminal-pane filtering and deterministic tiled/floating/suppressed ordering;
- compact zero/one-pane tabs and expanded multi-pane children;
- focused tiled/floating child selection and empty-title fallback;
- exact tab versus pane row targets;
- payload validation, timestamps, pane reuse, clear tombstones, cleanup, peer discovery, and snapshots;
- cell-aware truncation, wide characters, ellipsis, index/pane alignment, badge preservation, one-cell inset, and theme styling.

Keep runtime host calls out of these tests unless a proper mock layer is introduced; the non-WASM import stub exists only to link pure tests.

### Python bridge tests

Run `python3 -m unittest discover -s hooks/codex -p 'test_*.py'`. Add cases whenever lifecycle mapping, notification payload fields, forwarding syntax, or process cleanup changes.

### WASM and specification checks

Always pair host tests with a WASM build because host success does not prove the runtime target or `_start` module shape. For release work, build `--release` as well. The archived pane-aware completion record (`openspec/changes/archive/2026-07-17-nest-panes-under-tabs/tasks.md`) also records formatting, Clippy, strict OpenSpec validation, release installation, and live/headless verification. Repeat those categories when changing implementation or specs; exact formatter/Clippy/OpenSpec command lines are not checked into the repository, so do not assume undocumented aliases.

The current behavior contract is `openspec/specs/`. Completed proposals and rationale are under `openspec/changes/archive/`; there is no active change directory at the current merged `main` head.

## Runtime verification

Lifecycle, permissions, peer synchronization, layout safety, theme styling, and mouse focus require Zellij. The macOS PTY harness in `AGENTS.md` is:

```sh
( ( sleep 10 ) | env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
    TERM=xterm-256color script -q /tmp/e2e.log zellij -l zellij.kdl ) &
sleep 8 && zellij list-sessions
zellij kill-session <name>
```

Unset the Zellij variables so the process does not think it is nested. The input pipe may approve permissions with `y` and inject SGR mouse sequences. A minimum pane-aware pass verifies:

1. startup survives permission grant and renders a 32-column sidebar with no horizontal tab bar;
2. one-pane tabs stay compact, while a multi-pane tab lists all terminal panes and excludes plugin panes;
3. pane children follow tiled/floating/suppressed visual order and the focused visible-layer child is selected;
4. one-pane status appears on the tab, multi-pane statuses remain on exact children, and no aggregate/count appears;
5. a new tab's sidebar obtains current statuses from an existing peer;
6. a tab-row click switches tabs, while a pane-row click focuses that exact pane, including after scrolling;
7. long ASCII/wide names ellipsize, badges remain intact, and every normal-width row keeps its rightmost cell blank;
8. wheel overflow and keyboard tab switching operate on the flattened hierarchy.

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

### Client exits at startup

Keep `set_selectable(false)` deferred until the first event and keep `children` wrapped in `pane { ... }`. Test template changes in a fresh Zellij session, not only via hot reload. On macOS, inspect the Zellij log path described in `tasks/lessons.md` and vary one lifecycle/layout condition at a time.

### “Could not find exported function”

Confirm the explicit binary target and `register_plugin!(State)` remain. A reactor/cdylib build does not expose the `_start` entrypoint expected by Zellij.

## Release and maintenance notes

- Treat a Zellij upgrade as an ABI migration: update the binary, `zellij-tile`, lockfile, and runtime test environment together.
- Release remains a local mise build plus install/copy; no checked-in product CI publishes WASM.
- Keep baseline OpenSpec files synchronized with implemented behavior, then archive completed changes with their proposal/design/task evidence.
- `.github/workflows/openwiki-update.yml` updates documentation daily or on demand; it does not build, test, install, or release the plugin.
- The current merged branch is `main` at `0186450`, with a clean working tree before this documentation update. Recent history, unlike the initial wiki run, is available and records the status, badge, ellipsis, and pane-aware progression.
