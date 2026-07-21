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

For pure Rust or agent-bridge logic:

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
the state of every agent process, so restore only a state you know:

```sh
mise run status -- terminal_0 <agent-session-id> done Hub
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
   bridge tests under `hooks/codex/` and `hooks/claude/`.
4. Run `mise run test` during iteration and `mise run reload` for live UI
   feedback. Always rebuild before interpreting a runtime result.
5. Run the full local gate before review:

   ```sh
   mise run check
   ```

6. Perform fresh-session verification for changes involving layout,
   permissions, plugin lifecycle, the Zellij ABI, or selectability. Hot reload
   is insufficient for those paths.
7. After the behavior is accepted, archive the completed OpenSpec change on
   the feature branch. Archiving syncs its delta into `openspec/specs/` and
   moves the proposal, design, tasks, and evidence under
   `openspec/changes/archive/`:

   ```sh
   openspec archive <change-name> -y
   ```

   A user-visible change is not ready to merge while its completed OpenSpec
   directory remains active under `openspec/changes/<change-name>/`.
8. Rerun `mise run check` after archiving so the merged baseline and archived
   artifacts are validated together.
9. Update source documentation. Run `mise run docs` only after code and specs
   have stabilized, then review generated OpenWiki changes.
10. Commit the baseline-spec update, archived change, and generated docs on the
    same feature branch; push or update the PR, merge it, and return the
    checkout to a clean, synchronized `main`.

## Verification by change type

| Change | Minimum during iteration | Before merge |
| --- | --- | --- |
| Pure formatting or row model | `mise run test` | `mise run check`, live reload with native list selection and hierarchy |
| Agent hook/protocol | `mise run test` | `mise run check`, mixed-agent two-session status test, real permission/completion attention test |
| Pane focus/scroll/input | `mise run test`, live reload | `mise run check`, multi-tab/multi-pane test |
| Sidebar layout/resize | fresh disposable session | drag the sidebar/content boundary and compare pane geometry before/after |
| Layout/lifecycle/permissions | targeted tests | `mise run check`, fresh headless or interactive session |
| Zellij version/ABI | host tests plus WASM build | full fresh-session matrix; versions must match |
| Documentation only | link and terminology review | `git diff --check` |

The live pane-aware matrix should cover compact one-pane tabs, native `>`/`-`
hierarchy and selected surfaces, multi-pane child rows, independent statuses in
two tabs, exact pane clicks after scrolling, wide/long titles, right-edge
spacing, peer status synchronization, native attention for permission and final
answer events, and status cleanup after Codex or Claude Code exits.

## Agent-status performance invariants

The tab template creates one sidebar instance per tab, so lifecycle work must
remain linear as tabs grow:

- the untargeted `vertical-tab-agent-status` CLI pipe is already a Zellij
  broadcast; recipients apply it locally and must not relay it to peers;
- the lowest live sidebar plugin ID is the synchronization leader: a nonleader
  reports a newly observed focus transition only to that leader, and only the
  leader fans focus acknowledgements out, with deterministic turnover when it
  closes;
- late-joining sidebars recover through bounded peer snapshots rather than
  replaying every lifecycle update;
- repeated Codex `SessionStart` events refresh one PID-scoped watcher record;
  a non-blocking advisory lock allows only one long-lived watcher, and macOS
  uses `kqueue` process-exit notification instead of periodic polling;
- plugin snapshots live under a Zellij-server cache shard. Flat snapshots are
  read only for one-time migration, while dead host-journal directories are
  pruned only after their PID is demonstrably absent.

When changing these paths, test several tabs and repeated session-start events.
One external status should produce one local application per existing sidebar,
not an all-to-all relay, and repeated starts for one Codex PID should leave one
watcher process.

Reference measurements from the July 2026 macOS test host:

| Path | Observed bound |
| --- | --- |
| Two watcher starts for one Codex PID | One resident watcher (about 25 MiB RSS), then zero after process exit |
| Ordinary Python hook, 20 sequential invocations | About 1.18 seconds total, or 59 ms per invocation |
| One lifecycle update with `N` sidebars | `N` Zellij deliveries; no plugin relay |
| One focus transition with `N` sidebars | At most `3(N - 1)` peer messages: nonleader reports, leader focus fanout, and one acknowledgement fanout |

The Python interpreter remains the main per-session watcher memory cost. The
current change bounds that cost to one watcher per Codex process; replacing the
bridge with a smaller native helper remains a separate optimization.

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
- `test` — Rust unit tests plus Codex and Claude Code Python bridge tests
- `build` — debug WASM
- `spec` — strict OpenSpec validation
- `check` — formatting, tests, Clippy, debug WASM, OpenSpec, and diff hygiene
- `dev` / `reload` — fresh development session or fast hot reload
- `release` / `install` / `deploy` — gated release workflow
- `status` — explicitly republish known agent state after reload
- `docs` — regenerate OpenWiki after the implementation is stable

Crash-derived invariants and the headless PTY recipe remain in `AGENTS.md` and
`tasks/lessons.md`.
