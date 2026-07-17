---
type: Repository Guide
title: zellij-vertical-tab Quickstart
description: Entry point for understanding, building, testing, and changing the zellij-vertical-tab Rust/WASM plugin, including its source layout and critical Zellij 0.44 constraints.
resource: README.md
tags: [zellij, rust, wasm, plugin, quickstart]
---

# zellij-vertical-tab

`zellij-vertical-tab` is a Zellij plugin that replaces the usual horizontal tab bar with a fixed-width vertical list. Each row shows the tab's one-based switching index and name; the active row uses the user's Zellij theme. Users can click a row to switch tabs or scroll when the list exceeds the pane height (`README.md`, `src/main.rs`).

The codebase is intentionally small: all runtime behavior, pure domain helpers, and unit tests live in `src/main.rs`. Start with the [runtime architecture](architecture.md) to understand the event/state/render loop, then use the [development and operations guide](development.md) to build, test, install, or troubleshoot it.

## Fast path for engineers

### Prerequisites

- Zellij **0.44.3**.
- Rust with the `wasm32-wasip1` target.
- On the repository author's machine, Rust is managed by `mise`, so maintainer commands use `mise exec -- cargo ...` (`AGENTS.md`). Plain `cargo` is equivalent when a suitable toolchain is already on `PATH`.

The `zellij-tile` crate version must match the installed Zellij binary because their protobuf event schema is version-sensitive (`Cargo.toml`, `tasks/lessons.md`).

### Build and launch

```sh
rustup target add wasm32-wasip1
mise exec -- cargo test
mise exec -- cargo build --target wasm32-wasip1
zellij -l zellij.kdl
```

The debug plugin is generated at `target/wasm32-wasip1/debug/zellij_vertical_tab.wasm`. On first launch, grant `ReadApplicationState` and `ChangeApplicationState`; without the read permission, Zellij withholds tab updates and the pane remains blank. See [development and operations](development.md#build-run-and-install) for release installation and test procedures.

## What the plugin guarantees

- One rendered row per visible tab, with right-aligned one-based indices.
- Active-tab highlighting through Zellij's host-rendered `Text::selected()` theme style.
- Click-to-switch using the row's tab position.
- One-row wheel scrolling with `▲` and `▼` overflow markers.
- Automatic scroll adjustment when keyboard or external tab switching moves the active tab outside the visible window.
- A borderless, unselectable side pane in the supplied layout, inherited by every new tab.

These product rules are implemented by the state machine and pure formatting/window functions described in [architecture](architecture.md#domain-rules).

## Critical invariants

Preserve all of these when changing the plugin or layout:

1. Keep the project a **binary crate**. Zellij loads the WASM `_start` export generated through `register_plugin!`; a reactor/cdylib form does not provide the expected entrypoint.
2. Keep `zellij-tile` aligned with the Zellij binary version.
3. Do not call `set_selectable(false)` from `load()`. On Zellij 0.44, that can kill the client during startup when the plugin is in a `default_tab_template`; `update()` deliberately defers the call until the first event.
4. In the layout, keep template `children` wrapped inside `pane { children }`. A direct sibling relationship with the unselectable plugin pane was empirically observed to crash the session.
5. Keep the non-WASM `host_run_plugin_command` stub if host-target unit tests still link against `zellij-tile` host imports.

The reasons and data flow behind these constraints live in [architecture](architecture.md#runtime-and-layout-constraints); reproduction and diagnosis guidance lives in [development and operations](development.md#runbook-and-troubleshooting).

## Source map

| Path | Role | Start here when… |
| --- | --- | --- |
| `src/main.rs` | Complete plugin: lifecycle, state, event handling, rendering, pure helpers, and 12 unit tests | Changing behavior, interaction, rendering, or tests |
| `Cargo.toml` | Binary target, `zellij-tile` compatibility pin, size-focused release profile | Changing dependencies, Zellij version, artifact naming, or optimization |
| `Cargo.lock` | Resolved Rust dependency graph | Reviewing exact dependency resolution |
| `zellij.kdl` | Development `default_tab_template` with a 20-column plugin pane and status bar | Changing pane placement or reproducing runtime behavior |
| `README.md` | User-facing build, trial, installation, and feature documentation | Updating public usage instructions |
| `AGENTS.md` | Maintainer commands and crash-derived hard constraints | Preparing a code change or headless integration run |
| `tasks/lessons.md` | Empirical rationale, TUI test techniques, and failure analysis | Investigating compatibility or startup crashes |
| `tasks/todo.md` | Completed feature/e2e record and deferred feature ideas | Understanding progression and possible next work |
| `.github/workflows/openwiki-update.yml` | Scheduled/manual OpenWiki update pull request | Maintaining documentation automation |

There is no dedicated integration-test directory or release workflow. Unit tests are colocated with the implementation, while headless runtime verification is documented as an operator procedure.

## Repository progression and evidence limits

Checked-in notes describe a progression from scaffolding a Rust binary plugin, through implementation and 12 unit tests, to headless PTY verification of permission grant, rendering, new-tab inheritance, click switching, and active styling (`tasks/todo.md`). `tasks/lessons.md` records that controlled runtime experiments isolated the bin-crate requirement and the layout/selectability crash conditions.

The supplied repository root contains no `.git` metadata. `git status`, `git rev-parse`, and `git log` therefore fail, so commit chronology and blame evidence are unavailable in this documentation pass. Treat the task notes and inline comments as the available rationale, not as a replacement for recoverable git history.

## Documentation map

- [Architecture and domain model](architecture.md) — lifecycle, state, event flows, rendering, integrations, invariants, and extension points.
- [Development, testing, and operations](development.md) — build/install workflows, test strategy, headless runbook, troubleshooting, release concerns, and OpenWiki automation.

## Backlog

- **Automated CI artifact build** — source anchor: `tasks/todo.md` “Maybe later”; deferred because only documentation-update automation currently exists.
- **Future interactions (hover, new-tab row, right-click close)** — source anchor: `tasks/todo.md`; deferred because these are ideas, not implemented behavior.
- **Upstream Zellij crash report** — source anchor: `tasks/todo.md` and `tasks/lessons.md`; deferred because no upstream issue URL or resolution is recorded.
- **Standalone license file** — source anchor: `Cargo.toml` and `README.md`; package metadata says MIT, but no `LICENSE` file is present.
