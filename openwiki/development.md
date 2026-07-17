---
type: Engineering Runbook
title: "Development, Testing, and Operations"
description: Practical runbook for building, installing, testing, troubleshooting, and maintaining zellij-vertical-tab and its Zellij 0.44.3 integration.
resource: AGENTS.md
tags: [development, testing, operations, zellij, rust, wasm]
---

# Development, testing, and operations

This guide turns the [plugin architecture](architecture.md) into repeatable engineering workflows. For product scope and repository navigation, begin at the [quickstart](quickstart.md).

## Build, run, and install

### Toolchain

The plugin targets Zellij 0.44.3 and `wasm32-wasip1`. `Cargo.toml` pins `zellij-tile` to the same version. On the maintainer's machine, `mise` provides Rust, so use:

```sh
mise exec -- cargo test
mise exec -- cargo build --target wasm32-wasip1
mise exec -- cargo build --release --target wasm32-wasip1
```

If Rust is available globally, omit `mise exec --`. Add the WASI target once with `rustup target add wasm32-wasip1`.

Artifacts are:

- Debug: `target/wasm32-wasip1/debug/zellij_vertical_tab.wasm`
- Release: `target/wasm32-wasip1/release/zellij_vertical_tab.wasm`

The release profile enables LTO, optimizes for size, uses one codegen unit, and strips symbols (`Cargo.toml`). `target/` is ignored and should not be documented or committed as source.

### Development launch

After a debug build, run from the repository root:

```sh
zellij -l zellij.kdl
```

Approve the permission prompt on first load. The layout loads the debug artifact from a relative `file:` path, so launching elsewhere can make the path resolve incorrectly.

For a running development session, README documents:

```sh
zellij action start-or-reload-plugin \
  file:target/wasm32-wasip1/debug/zellij_vertical_tab.wasm
```

Rebuild before each reload; `tasks/lessons.md` records that a stale WASM artifact once produced misleading integration results.

### Everyday installation

```sh
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/zellij_vertical_tab.wasm \
  ~/.config/zellij/plugins/
```

Then add a `default_tab_template` equivalent to the README example under `~/.config/zellij/layouts/default.kdl`. Preserve the nested pane around `children`:

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

The default template makes new tabs inherit the sidebar. It replaces the default horizontal tab-bar unless that built-in plugin is also retained.

## Testing strategy

### Host unit tests

Twelve tests are colocated in `src/main.rs`. They cover:

- no-overflow and zero-height windows;
- absent, above-window, below-window, and already-visible active tabs;
- offset repair when tabs shrink;
- boundary clamping;
- padding and truncation;
- single- and double-digit row alignment and overflow markers.

Run:

```sh
mise exec -- cargo test
```

A non-WASM `host_run_plugin_command` symbol stub allows these host-target tests to link. Tests deliberately exercise pure helpers only; calling runtime host APIs in host tests would require a different mocking strategy.

When changing viewport or formatting rules, add table-like edge cases around zero rows, fewer tabs than rows, last-page offsets, changing active position, narrow widths, and multibyte names. The [architecture domain rules](architecture.md#domain-rules) define the intended behavior.

### WASM build check

Unit-test success does not prove that the plugin compiles to the runtime target or exports the command entrypoint. Always pair it with:

```sh
mise exec -- cargo build --target wasm32-wasip1
```

For release work, also build `--release`. The checked-in completion notes say `_start`, `load`, `update`, `render`, `pipe`, and `plugin_version` were previously verified, but there is no checked-in automated export assertion.

### Headless runtime verification

Lifecycle, permissions, layout stability, theme styling, and mouse behavior require a real Zellij process. `AGENTS.md` documents a macOS PTY harness:

```sh
( ( sleep 10 ) | env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
    TERM=xterm-256color script -q /tmp/e2e.log zellij -l zellij.kdl ) &
sleep 8 && zellij list-sessions
zellij kill-session <name>
```

The environment variables are removed so the process does not think it is nested in or attaching to another session. The input pipe can send `y` for permission approval and SGR mouse sequences for clicks. Discover the generated session name from `zellij list-sessions`; `zellij -s <name>` is attach semantics and does not create a fresh session.

A minimum runtime regression pass should verify:

1. The session remains alive after startup and permission grant.
2. The vertical list renders and the horizontal tab bar is absent as intended.
3. Creating a tab inherits the sidebar and highlights the new active row.
4. Clicking the previous row switches back.
5. Enough tabs to overflow show markers and wheel scrolling respects boundaries.
6. Keyboard tab switching moves the visible window to include the active tab.

`tasks/todo.md` records that startup stability, permission grant, rendering, click switching, and active styling passed in the final historical PTY run. That record is useful regression evidence, but it is not an automated test suite.

## Runbook and troubleshooting

### Blank pane after load

1. Check whether `ReadApplicationState` was granted. Without it, Zellij silently withholds `TabUpdate`; task notes say server logs contain per-event permission denial.
2. Confirm the plugin path. Permission grants are cached by location, so a changed path can require approval again.
3. Confirm `zellij-tile` and the Zellij binary are both 0.44.3.
4. Confirm the debug or installed WASM was rebuilt and exists at the KDL location.

### “Could not find exported function”

Confirm `Cargo.toml` still declares a binary target using `src/main.rs` and `register_plugin!(State)` remains. A cdylib/reactor build does not provide the `_start` export expected by Zellij.

### Client exits during startup

Check both crash-derived constraints before changing application logic:

- `set_selectable(false)` must remain deferred until the first `update()` event.
- Layout `children` must remain wrapped in its own `pane` when adjacent to the unselectable plugin pane.

On macOS, inspect `$TMPDIR/zellij-501/zellij-log/zellij.log`; `tasks/lessons.md` notes that “Bye from Zellij!” indicates a clean router-ended client exit, so inspect preceding messages for the trigger. Reproduce with a controlled PTY matrix and change one layout/API variable at a time.

### Mouse click selects the wrong tab

Trace the coordinate conventions in [architecture](architecture.md#click-to-switch): mouse content rows are zero-based, `scroll_offset` is a vector index, and `switch_tab_to` expects `TabInfo.position + 1`. Changes that add headers, separators, or action rows must introduce an explicit visual-row mapping rather than reusing direct indexing.

### Highlight does not span the row

Ensure `format_row` still pads output to the full pane width and active rows still receive `Text::selected()`. Styling is resolved by Zellij's theme; hard-coded terminal colors would bypass the intended integration.

## Compatibility and release notes

- Treat a Zellij upgrade as an integration change: update the binary, `zellij-tile`, lockfile, and runtime test environment together.
- Re-run host tests, debug WASM build, and the headless scenario after any version update.
- Preserve the binary-module shape and inspect exports if packaging changes.
- Test layout modifications in `default_tab_template`, not only an explicit one-off tab, because startup geometry caused the known crash.
- No GitHub Actions workflow currently builds/tests or publishes WASM. Release is a local build-and-copy process.
- Package metadata and README state MIT, but the repository has no standalone `LICENSE` file.

## Documentation automation

`.github/workflows/openwiki-update.yml` runs daily at 08:00 UTC and on manual dispatch. It checks out the repository, installs OpenWiki with Node.js 22, executes `openwiki code --update --print`, and opens/updates an `openwiki/update` pull request containing the wiki and OpenWiki-related repository files.

The workflow is a documentation integration, not a product CI check: it does not install Rust, run tests, build WASM, or exercise Zellij. Credentials are supplied through GitHub Actions secrets; never copy their values into documentation.

## Verification status for this wiki pass

The source tree, configuration, existing notes, workflow, lockfile version, 12 test declarations, and generated debug/release artifact presence were inspected. Fresh commands could not be run because `mise`, `cargo`, `rustc`, and `zellij` were unavailable in the execution environment. Git history also could not be inspected because the supplied root has no `.git` directory. Future updates should replace this limitation with current command and commit evidence when the repository metadata and toolchain are available.
