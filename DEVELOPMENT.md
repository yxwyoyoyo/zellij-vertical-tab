# Development workflow

This project uses mise as the command entrypoint. The workflow separates the
fast rendering loop from the full release gate so stale WASM artifacts and
startup-only Zellij failures are harder to miss.

## Bootstrap

```sh
mise trust
mise install
mise run setup
```

The project pins Rust 1.97.1 and Node 26.5.0. `setup` installs the
`wasm32-wasip1` target plus the OpenSpec and OpenWiki CLIs used by the
repository.

## Daily loop

For pure Rust or bridge logic:

```sh
mise run test
```

For visual and interaction changes, use a dedicated Zellij development
session:

```sh
mise run dev
```

After an edit, rebuild and reload from inside that session:

```sh
mise run reload
```

From another terminal, name the target session:

```sh
mise run reload -- Hub
# or: ZELLIJ_DEV_SESSION=Hub mise run reload
```

Hot reload resets the plugin's in-memory agent records. It cannot safely infer
the state of every Codex process, so restore only a state you know:

```sh
mise run status -- terminal_0 <codex-session-id> done Hub
```

Starting a new prompt also republishes state through the normal Codex hooks.

Sidebar resizing is startup-sensitive. With Zellij mouse handling enabled (the
default), drag the tiled boundary between the sidebar and content to change the
sidebar width. Pane frames are optional: showing them makes the boundary
visible, while hiding them leaves the same one-cell drag target. The width
belongs to that tab only and a new tab starts at the layout's `13%` default.
Test layout or resizing changes in a disposable fresh session; hot reload does
not reconstruct pane geometry.

## Feature loop

1. Start from a clean, current `main` and create a feature branch.
2. For user-visible behavior, create an OpenSpec change before implementation.
   Keep proposal, delta specs, design, and tasks aligned as decisions change.
3. Implement one coherent slice and add pure host tests beside `src/main.rs` or
   bridge tests under `hooks/codex/`.
4. Run `mise run test` during iteration and `mise run reload` for live UI
   feedback. Always rebuild before interpreting a runtime result.
5. Run the full local gate before review:

   ```sh
   mise run check
   ```

6. Perform fresh-session verification for changes involving layout,
   permissions, plugin lifecycle, the Zellij ABI, or selectability. Hot reload
   is insufficient for those paths.
7. After the behavior is accepted, sync delta specs into `openspec/specs/`,
   archive the completed change, and rerun `mise run check`.
8. Update source documentation. Run `mise run docs` only after code and specs
   have stabilized, then review generated OpenWiki changes.
9. Commit a scoped branch, open a PR, merge it, and return the checkout to a
   clean, synchronized `main`.

## Verification by change type

| Change | Minimum during iteration | Before merge |
| --- | --- | --- |
| Pure formatting or row model | `mise run test` | `mise run check`, live reload |
| Codex hook/protocol | `mise run test` | `mise run check`, two-session status test |
| Pane focus/scroll/input | `mise run test`, live reload | `mise run check`, multi-tab/multi-pane test |
| Sidebar layout/resize | fresh disposable session | drag the sidebar/content boundary and compare pane geometry before/after |
| Layout/lifecycle/permissions | targeted tests | `mise run check`, fresh headless or interactive session |
| Zellij version/ABI | host tests plus WASM build | full fresh-session matrix; versions must match |
| Documentation only | link and terminology review | `git diff --check` |

The live pane-aware matrix should cover compact one-pane tabs, multi-pane child
rows, independent statuses in two tabs, exact pane clicks after scrolling,
wide/long titles, right-edge spacing, peer status synchronization, and status
cleanup after Codex exits.

## Release and install

Build a release only after the complete local gate:

```sh
mise run release
```

Install it for everyday use:

```sh
mise run install
```

Install and reload a running session in one command:

```sh
mise run deploy -- Hub
```

For startup-sensitive changes, start a new session after installation instead
of relying on `deploy`.

## Available tasks

```sh
mise tasks ls
```

- `setup` — install the WASM target and pinned project CLIs
- `test` — Rust unit tests plus Python bridge tests
- `build` — debug WASM
- `spec` — strict OpenSpec validation
- `check` — formatting, tests, Clippy, debug WASM, OpenSpec, and diff hygiene
- `dev` / `reload` — fresh development session or fast hot reload
- `release` / `install` / `deploy` — gated release workflow
- `status` — explicitly republish known agent state after reload
- `docs` — regenerate OpenWiki after the implementation is stable

Crash-derived invariants and the headless PTY recipe remain in `AGENTS.md` and
`tasks/lessons.md`.
