# zellij-vertical-tab — todo

## Done

- [x] Upgrade zellij 0.43.1 → 0.44.3 (brew)
- [x] Install Rust 1.97.1 via mise + `wasm32-wasip1` target
- [x] Scaffold (Cargo.toml bin crate, zellij.kdl, .gitignore)
- [x] Plugin: vertical tab list, index numbers, theme highlight, click-to-switch, wheel scroll, ▲/▼ overflow markers, active-tab auto-follow
- [x] 12 unit tests green (host); wasm exports verified (`_start`, `load`, `update`, `render`, `pipe`, `plugin_version`)
- [x] Headless pty e2e: session stable, permission grant, tab list rendered, click switches tab, active row highlighted
- [x] README.md, AGENTS.md

## Review (verification summary)

End-to-end proof from the final pty run (session `oblong-ukulele`):

- Started with `zellij -l zellij.kdl`, granted permission with `y`
- `Ctrl t n` opened Tab #2 (became active), SGR mouse click on row 1 switched back
- Final screen: ` 1 Tab #1` with selected theme background, ` 2 Tab #2` plain →
  render + click-to-switch + highlight all confirmed working
- Session still alive 20s in (no client crash)

### Zellij pitfalls hit (details in tasks/lessons.md)

1. cdylib plugin fails to load: "could not find exported function" → must be a bin crate.
2. `set_selectable(false)` in `load()` inside `default_tab_template` kills the client → deferred to first event.
3. `children` as direct sibling of the unselectable plugin pane crashes the session → wrap in `pane { ... }`.

## Maybe later

- [ ] Report the `children`-sibling crash upstream (zellij issue)
- [ ] Hover highlight (`Mouse::Hover`)
- [ ] `+` new-tab row, right-click to close tab
- [ ] CI build (GitHub Actions → wasm artifact)
