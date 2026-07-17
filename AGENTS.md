# AGENTS

Zellij plugin (Rust → wasm32-wasip1) rendering tabs vertically in a 20-col side pane.

## Commands

This machine has no system cargo; Rust is managed by mise — prefix cargo with `mise exec --`:

```sh
mise exec -- cargo test                                   # unit tests (host target)
mise exec -- cargo build --target wasm32-wasip1           # debug wasm
mise exec -- cargo build --release --target wasm32-wasip1 # release wasm
zellij -l zellij.kdl                                      # run the dev layout (from repo root)
```

Headless end-to-end test (macOS `script` gives a pty; session must not think it's nested):

```sh
( ( sleep 10 ) | env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
    TERM=xterm-256color script -q /tmp/e2e.log zellij -l zellij.kdl ) &
sleep 8 && zellij list-sessions   # session must be alive
zellij kill-session <name>
```

Keystrokes can be injected into the pipe (e.g. `printf 'y'` for the permission prompt,
`printf '\033[<0;2;1M\033[<0;2;1m'` for an SGR left-click at col 2 row 1).

## Hard constraints (all learned from crashes)

- **`zellij-tile` version must equal the zellij binary version** (protobuf event schema).
- Plugin must be a **bin crate** (`src/main.rs`): zellij requires the wasm export `_start`;
  cdylib/reactor modules fail with "could not find exported function".
- **Never call `set_selectable(false)` in `load()`** — in a `default_tab_template` it kills
  the client at startup on zellij 0.44. Defer to the first event (see `update()`).
- In layouts, the template's **`children` must be wrapped in `pane { ... }`** when a sibling
  pane is an unselectable plugin; a direct `children` sibling crashes the session.
- Host builds (`cargo test`) need the `host_run_plugin_command` stub at the top of `src/main.rs`.

## Structure

- `src/main.rs` — the whole plugin (state, update, render, pure helpers + unit tests)
- `zellij.kdl` — dev layout (left sidebar + status-bar, no horizontal tab-bar)
- `tasks/` — todo/lessons notes

<!-- OPENWIKI:START -->

## OpenWiki

This repository uses OpenWiki for recurring code documentation. Start with `openwiki/quickstart.md`, then follow its links to architecture, workflows, domain concepts, operations, integrations, testing guidance, and source maps.

The scheduled OpenWiki GitHub Actions workflow refreshes the repository wiki. Do not hand-edit generated OpenWiki pages unless explicitly asked; prefer updating source code/docs and letting OpenWiki regenerate.

<!-- OPENWIKI:END -->
