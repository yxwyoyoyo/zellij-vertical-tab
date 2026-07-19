---
type: Repository Guide
title: zellij-vertical-tab Quickstart
description: Entry point for the pane-aware zellij-vertical-tab Rust/WASM plugin, including its native nested-list tab and pane hierarchy, per-pane Codex status, mouse-resizable layout, source map, and maintainer workflows.
resource: README.md
tags: [zellij, rust, wasm, codex, quickstart]
---

# zellij-vertical-tab

`zellij-vertical-tab` replaces Zellij's horizontal tab bar with a borderless vertical sidebar. Every tab has a native top-level `>` row; tabs with zero or one terminal pane remain compact, while tabs with multiple terminal panes gain always-visible indented `-` children. Zellij's nested-list component supplies the hierarchy, typography, and theme styling. The hierarchy is adaptive rather than collapsible, and plugin panes such as the sidebar and status bar are never shown as children (`README.md`, `src/main.rs`).

The [architecture guide](architecture.md) explains the flattened row model, pane-owned Codex state, synchronization, rendering, and click dispatch. The [development and operations guide](development.md) covers mise-managed build, test, release, install, hot reload, and runtime checks.

## Fast path

### Requirements

- Zellij **0.44.3** and matching `zellij-tile = 0.44.3`.
- [`mise`](development.md#bootstrap-and-task-entrypoints), which pins Rust **1.97.1** and Node **26.5.0** and installs the `wasm32-wasip1` target plus repository CLIs (`mise.toml`, `DEVELOPMENT.md`).
- A Nerd Font for the four Codex status glyphs.

```sh
mise trust
mise install
mise run setup # once: WASM target, OpenSpec, and OpenWiki
mise run test  # Rust host tests + Python bridge tests
mise run dev   # build debug WASM and launch zellij.kdl
```

The development layout loads `target/wasm32-wasip1/debug/zellij_vertical_tab.wasm` into a flexible pane that starts at **13%** of the terminal width. With normal Zellij mouse handling, start a fresh session and drag the tiled boundary between the sidebar and content to resize the sidebar. Pane frames are optional: showing them makes the boundary visible, while hiding them leaves the same one-cell drag target. Zellij owns that width per tab: it is not persisted or synchronized, and each new tab starts again at 13%. On first use, approve `ReadApplicationState`, `ChangeApplicationState`, `ReadCliPipes`, and `MessageAndLaunchOtherPlugins`; the last two support Codex messages and synchronization among the sidebar instance created in each tab. See [the layout architecture](architecture.md#runtime-and-layout-constraints) and [fresh-session verification](development.md#runtime-verification) for the boundary and checks.

## User-visible contract

- **Adaptive native hierarchy:** every tab contributes a top-level `>` list item, used as the sole leading marker without repeating the tab number. More than one terminal pane adds one indented `-` child item per terminal pane; zero or one does not. Children are ordered tiled, floating, then suppressed, with each layer ordered by `y`, `x`, and pane ID.
- **Status ownership:** status is keyed by terminal pane ID. A one-pane tab shows its pane's glyph on the compact tab row. A multi-pane tab shows each glyph only on its owning pane child; the parent has no aggregate glyph or pane count.
- **Selection:** the active tab and the focused child of the active multi-pane tab use Zellij's native selected-list palette across the fitted row; all other rows use the native unselected-list palette.
- **Exact clicks:** a valid tab-row click calls `switch_tab_to(position + 1)`; a pane-row click calls `focus_terminal_pane(id, false, false)`, allowing Zellij to switch tab/layer and focus that exact terminal pane. Clicks outside rendered rows do nothing.
- **Shared viewport:** rendering, wheel scrolling, overflow markers, and click mapping consume one flattened row vector. Wheel input moves one hierarchy row; keyboard tab changes minimally move the window to reveal the active tab.
- **Cell-aware rows:** after reserving three cells of native top-level chrome or five cells of child chrome, tab names and pane titles are measured in terminal cells and receive `…` when truncated. Every row fills the available width for selection styling and, whenever its required prefix or badge still fits, reserves a **one-cell right inset**. A status glyph is right-aligned before that uncolored cell.

The hierarchy, native presentation, interaction, and status rules are canonicalized in `openspec/specs/vertical-tab-sidebar/spec.md` and `openspec/specs/agent-status/spec.md`; the accepted presentation rationale is archived under `openspec/changes/archive/2026-07-19-refresh-zellij-native-sidebar-ui/` and implemented in `src/main.rs`.

## Critical invariants

1. Keep the plugin a binary crate: Zellij expects the command-module `_start` export generated through `register_plugin!`.
2. Keep `zellij-tile` aligned with the installed Zellij binary.
3. Keep `set_selectable(false)` out of `load()`; it is deliberately deferred to the first event to avoid a Zellij 0.44 startup failure.
4. Keep layout `children` wrapped in `pane { children }` beside the unselectable plugin pane.
5. Keep the non-WASM `host_run_plugin_command` stub while host tests link against `zellij-tile` imports.
6. Derive rendering and clicks from the same `SidebarRow` sequence; parallel row arithmetic would make scrolled pane clicks unsafe.

See [architecture constraints](architecture.md#runtime-and-layout-constraints) for rationale and [the runbook](development.md#runbook) for diagnosis.

## Source map

| Path | Role | Start here when… |
| --- | --- | --- |
| `src/main.rs` | Plugin state, Zellij lifecycle, adaptive rows, status synchronization, formatting, input, and Rust unit tests | Changing runtime or UI behavior |
| `hooks/codex/` | Dependency-free lifecycle/completion bridges, hook template, and Python tests | Changing Codex publication or installation |
| `openspec/specs/` | Current behavior contracts for the sidebar and agent status | Checking intended product behavior |
| `openspec/changes/archive/` | Archived proposals, designs, deltas, and completion evidence | Understanding why status, badges, ellipsis, or pane hierarchy changed |
| `zellij.kdl` | Development template with 13% flexible sidebar, sibling content pane, and status bar | Changing layout or launching locally |
| `Cargo.toml` | Binary target, ABI-sensitive dependency pin, and size-focused release profile | Changing packaging or dependencies |
| `mise.toml` | Pinned tools and reproducible setup, test, build, check, reload, release, install, deploy, status, and docs tasks | Running or changing maintainer automation |
| `scripts/` | Safe session-aware plugin reload and explicit known-state status republication | Changing runtime helpers or reload recovery |
| `README.md` | User installation, Codex setup, and behavior | Updating public instructions |
| `AGENTS.md` and `DEVELOPMENT.md` | Canonical commands, feature/release workflow, PTY runbook, and crash-derived constraints | Preparing or validating a change |
| `.github/workflows/openwiki-update.yml` | Scheduled/manual documentation update PR | Maintaining wiki automation |

## Repository progression

Git history shows the initial vertical list followed by four specification-driven increments on 2026-07-17: per-pane Codex lifecycle status, theme-aligned status glyphs, cell-aware name ellipsis, and finally pane-aware rows in merged commit `0186450`. Each completed change was moved under `openspec/changes/archive/`, while its resulting contract was merged into `openspec/specs/`. The latest change intentionally replaced lossy multi-pane aggregation with exact pane ownership while preserving the existing wire protocol and cross-sidebar synchronization.

## Documentation map

- [Architecture and domain model](architecture.md) — lifecycle, state ownership, hierarchy, synchronization, rendering, and integrations.
- [Development, testing, and operations](development.md) — current commands, Codex setup, release/install/hot reload, checks, and troubleshooting.

## Backlog

- **Automated product CI/artifact publishing** — source anchor: `.github/workflows/` and `tasks/todo.md`; deferred because the only checked-in workflow updates OpenWiki.
- **Future interactions (hover, new-tab row, right-click close)** — source anchor: `tasks/todo.md`; deferred because these remain ideas rather than implemented behavior.
- **Upstream Zellij startup-crash report** — source anchor: `tasks/lessons.md`; deferred because no upstream issue or resolution is recorded.
- **Standalone license file** — source anchor: `Cargo.toml` and `README.md`; metadata says MIT but no `LICENSE` file is checked in.
