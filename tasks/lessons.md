# Lessons

Patterns worth remembering from this build.

## Zellij plugin development (hard-won, all empirically verified on 0.44.3)

1. **Plugins must be bin crates.** Zellij's `plugin_loader` does
   `instance.get_typed_func(&mut store, "_start")` — cdylib/reactor modules don't export
   `_start` and fail with "could not find exported function" (empty name in the message).
   `register_plugin!` generates `fn main()` for this reason. Built-in plugins are all `main.rs`.
2. **`set_selectable(false)` is layout-sensitive.** It is safe in flat bar-style template
   panes (like `zellij:tab-bar`) and in explicit layouts, but kills the session when the
   plugin pane is a sibling of a template's `children` inside a vertical split. Two mitigations:
   defer the call past startup (first event, not `load()`) AND wrap `children` in `pane { ... }`.
   Root cause is zellij-side layout geometry, not the plugin.
3. **`zellij-tile` version == zellij binary version.** The event protocol is protobuf;
   mismatch fails at runtime (PLUGIN_MISMATCH), not load time.
4. **Permissions are cached on disk** (`<cache dir>/permissions.kdl`, keyed by plugin location).
   Without `ReadApplicationState`, `TabUpdate` is silently withheld (server logs "permission
   denied" per event) and the plugin renders nothing.
5. **Unselectable panes DO receive `Mouse` events** (line = 0-based content row, 1-based tab
   indices for `switch_tab_to`) — same as the built-in tab-bar.
6. **`Text`/`print_text_with_coordinates` = `ztext` APC sequences**; zellij applies the user
   theme host-side, so `.selected()` needs no `ModeUpdate` plumbing. Pad the row string to
   full width for a row-wide highlight.

## Testing TUIs headlessly (macOS)

- `( sleeps | env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID TERM=xterm-256color script -q out.log zellij -l layout.kdl ) &`
  gives a pty, accepts injected keystrokes (incl. SGR mouse), and `zellij list-sessions` /
  `kill-session` manage lifecycle. Strip ANSI from `list-sessions` output before using names.
- `zellij -s <name>` does NOT create a session (attach semantics) — use `-l` and discover
  the random name. A stale `ZELLIJ_SESSION_NAME` env makes `-l` fail with "There is no
  active session!" — unset it.
- `pyte` renders the captured log into a final screen for assertions (pip in a throwaway venv).
- zellij logs live in `$TMPDIR/zellij-501/zellij-log/zellij.log`; "Bye from Zellij!" in the
  client log = clean exit (router ended), look backwards for the cause.

## Process

- When integration testing surfaces a crash, bisect ONE variable at a time and keep a result
  matrix — seven controlled pty runs isolated a layout×API interaction that no amount of
  log reading alone would have proven.
- Rebuild before every e2e run: one bisect round tested a stale wasm because the build
  failed (a brace slipped during an edit) and produced a misleading data point.
