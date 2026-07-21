## Why

The sidebar currently receives lifecycle state only from Codex. Claude Code exposes equivalent user-level lifecycle hooks, but no bridge translates them into the existing pane-scoped Zellij protocol, so Claude sessions never show working, waiting, done, or idle badges.

## What Changes

- Add a dependency-free Claude Code hook bridge and user-level settings template.
- Translate Claude session, prompt, tool, permission, completion, and session-end events into the existing version-1 agent-status protocol.
- Use Claude's prompt identifier as the turn identity so completion remains terminal within one prompt while a later prompt resumes working.
- Journal Claude events in the same server- and pane-scoped host store used by Codex, including an explicit clear on `SessionEnd`.
- Allow detached-event recovery to use either the installed Codex bridge or the installed Claude bridge as the snapshot helper.
- Keep badge rendering agent-neutral: Claude and Codex use the same icons, colors, focus acknowledgement, and per-pane isolation without an agent-name prefix.
- Return Claude's supported terminal bell sequence for visible permission requests and completed responses so Zellij can retain native tab attention.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Accept equivalent lifecycle publication from Claude Code and document installation, cleanup, persistence, and mixed-agent behavior.

## Impact

- `hooks/claude/`: Claude lifecycle and notification bridge, settings template, and tests.
- `hooks/codex/status_store.py`: recognize the additional normalized Claude tool events.
- `src/main.rs`: host-snapshot helper fallback for Claude-only installations.
- `mise.toml`, README, DEVELOPMENT, OpenWiki, and OpenSpec: test and document both supported agents.
