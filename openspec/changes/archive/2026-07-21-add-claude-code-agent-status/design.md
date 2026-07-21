## Context

Claude Code command hooks receive JSON on stdin and expose `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PostToolUseFailure`, `PermissionDenied`, `Stop`, and `SessionEnd`. Their common payload includes a session ID and, after the first prompt, a prompt ID. The existing external protocol and Rust state are already agent-neutral and keyed by terminal pane, session, timestamp, and optional turn identity.

## Goals / Non-Goals

**Goals:**

- Give Claude Code sessions the same pane-scoped idle, working, waiting, and done presentation as Codex.
- Preserve terminal completion, focus acknowledgement, pane reuse, detach recovery, and mixed Codex/Claude isolation without changing the wire protocol.
- Make user-level installation additive and preserve unrelated Claude settings and hooks.
- Clear normal Claude exits and interactive session switches promptly through `SessionEnd`.
- Raise native Zellij attention when Claude needs permission or finishes responding.

**Non-Goals:**

- Do not display an agent-name prefix or distinguish Claude from Codex visually.
- Do not infer additional semantic states from assistant text or transcript contents.
- Do not add a resident exit watcher for Claude; `SessionEnd` and Zellij pane removal cover normal cleanup, while abrupt process death in a still-open pane remains best-effort.
- Do not rewrite existing user settings automatically from repository code.

## Decisions

### Reuse the agent-neutral version-1 protocol

The Claude bridge publishes the existing fields: `version`, `pane_id`, `session_id`, `state`, `updated_at_ms`, `event`, and optional `turn_id`. It copies Claude's `prompt_id` into `turn_id`. This lets the current Rust validator, record ordering, focus acknowledgement, persistence, and rendering operate unchanged and allows Codex and Claude sessions to coexist in different panes.

### Map only observable lifecycle boundaries

`SessionStart` maps to idle. `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, and `PermissionDenied` map to working. `PermissionRequest` maps to waiting because Claude documents that it fires when a permission dialog is about to be shown, including when automation ultimately requires user review. `Stop` maps to done. `SessionEnd` emits a matching-session clear tombstone.

This avoids transcript heuristics. A delayed event from a completed prompt cannot reopen `done`; a different prompt ID or the next `UserPromptSubmit` can.

### Share the durable store without coupling installations

The repository keeps one `status_store.py` implementation under `hooks/codex/`, and installation copies that same file beside the Claude bridge. Both bridges write the same cache root and versioned records. The plugin's fixed recovery command tries the Codex helper first and then the Claude helper, so either integration can serve a snapshot without requiring the other to be installed.

### Merge hooks into user settings

The repository provides a complete template for the supported events, but documentation instructs users to merge its `hooks` entries into `~/.claude/settings.json` rather than overwrite that file. The hook command uses an absolute home-relative shell path and remains best-effort: malformed input, missing Zellij variables, storage errors, and pipe failures never block Claude.

### Emit terminal attention from lifecycle hooks

Claude Code 2.1.141 and newer accepts a top-level `terminalSequence` in command-hook JSON output and emits that sequence through its own terminal. The bridge returns a single BEL for `PermissionRequest` and final `Stop` events, allowing Zellij to retain its native tab-scoped bell state and the sidebar to render the existing attention icon. A `Stop` with `stop_hook_active: true` does not ring because Claude is continuing rather than presenting a final answer. Other lifecycle events emit no terminal sequence.

This reuses the status hook instead of adding Claude's general `Notification` event, preventing duplicate alerts and aligning attention with the same observable boundaries that publish `waiting` and `done`. The JSON response does not approve, deny, block, or otherwise alter Claude's lifecycle decision. Pane status remains pane-scoped; Zellij's native bell ownership remains tab-scoped.

## Risks / Trade-offs

- [Claude terminates abnormally without `SessionEnd` while the pane remains open] -> The last record can remain until the pane closes or a new session starts; document this best-effort edge rather than adding another Python watcher.
- [Codex and Claude session IDs collide] -> Records are isolated by terminal pane, and a new session in one pane replaces the old pane record.
- [A user already has Claude hooks] -> Require structural merge and preserve all existing hook groups and handlers.
- [Claude changes its hook schema] -> Validate required common fields strictly and ignore unknown fields/events without affecting the TUI.
- [Older Claude versions ignore terminal hook output] -> Document the 2.1.141 minimum for native attention while keeping status publication compatible.

## Migration Plan

Install the Claude bridge and shared store under `~/.claude/hooks`, merge the template's hook groups into `~/.claude/settings.json`, then start a new Claude Code session. Existing Codex installation and protocol records remain compatible. Rollback removes only the added Claude hook handlers and files.

## Open Questions

None. Richer agent identity and notification-only background-agent events outside the visible permission/completion lifecycle remain separate capabilities.
