# Agent status integration

The sidebar can display Codex and Claude Code lifecycle state for the exact
Zellij terminal pane that owns each session. Both integrations use the same
agent-neutral protocol and visual states, so they can run concurrently without
an agent-name prefix.

## Status behavior

| Badge | State | Theme color |
| --- | --- | --- |
| `` | Session is idle | Dimmed |
| `` | Agent is working | Cyan emphasis |
| `` | Agent is waiting for permission | Orange emphasis |
| `` | Agent delivered an answer | Success color |
| `` | Zellij retained attention for the tab | Orange emphasis |

Status is tracked per terminal pane. A tab with one terminal pane keeps the
badge on its compact tab row. A tab with multiple terminal panes shows pane
titles beneath the parent and puts each badge on its owning pane row; the parent
does not duplicate an aggregate badge.

Returning to a pane whose current state is `done` acknowledges that exact result
and presents it as idle. A completion that arrives while the pane remains
focused stays `done` until the user leaves and returns. Working and waiting
states are never acknowledged by focus, and acknowledgement remains separate
from Zellij's native bell state.

## Codex

Install the dependency-free bridge and user-level hook template:

```sh
mkdir -p ~/.codex/hooks
install -m 755 hooks/codex/agent_status.py ~/.codex/hooks/agent_status.py
install -m 755 hooks/codex/agent_notify.py ~/.codex/hooks/agent_notify.py
install -m 644 hooks/common/agent_bridge.py ~/.codex/hooks/agent_bridge.py
install -m 644 hooks/common/status_store.py ~/.codex/hooks/status_store.py
install -m 644 hooks/codex/hooks.json ~/.codex/hooks.json
```

If `~/.codex/hooks.json` already exists, merge the new entries instead of
replacing the file. Configure the external completion notifier in
`~/.codex/config.toml`, replacing `/Users/you` with your absolute home path:

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py"]

[tui]
notifications = ["agent-turn-complete", "approval-requested"]
notification_method = "bel"
notification_condition = "always"
```

If another notifier is already configured, preserve it by forwarding its
command and arguments before the final `--`:

```toml
notify = ["/usr/bin/python3", "/Users/you/.codex/hooks/agent_notify.py", "--forward", "/path/to/existing-notifier", "existing-arg", "--"]
```

Start a new Codex session after changing the configuration. Open `/hooks` in
Codex and trust the user hook when prompted.

Codex invokes the lifecycle bridge at session, prompt, pre-tool, permission,
post-tool, and stop boundaries. A manually reviewed `PermissionRequest`
publishes `waiting`. During auto-review the bridge uses the reviewer identity in
the transcript to retain `working`; if that context is unavailable, it
conservatively publishes `waiting`. `PostToolUse` restores `working` after an
approved tool returns control to the agent. The external notifier covers
completion paths, such as code review, that can omit `Stop`.

Each record may include the normalized lifecycle event and Codex turn ID. Within
a turn, `done` is terminal: a delayed tool event cannot reopen it, while
`UserPromptSubmit` begins the next turn as `working`. Legacy records without the
optional metadata remain readable. Because Codex has no session-exit hook, the
session-start handler launches a detached watcher that publishes `clear` when
that Codex process exits.

The TUI notification settings are independent from lifecycle status. `always`
is required because switching Zellij panes does not make Codex's focus detector
report `unfocused` in every terminal setup. BEL lets Zellij retain attention on
an inactive owning tab until Zellij acknowledges it.

## Claude Code

Install the Claude adapter and common runtime:

```sh
mkdir -p ~/.claude/hooks
install -m 755 hooks/claude/agent_status.py ~/.claude/hooks/agent_status.py
install -m 644 hooks/common/agent_bridge.py ~/.claude/hooks/agent_bridge.py
install -m 644 hooks/common/status_store.py ~/.claude/hooks/status_store.py
```

Merge the `hooks` object from `hooks/claude/settings.json` into
`~/.claude/settings.json`. Preserve existing environment, model, theme,
permissions, plugins, and hook handlers; do not replace the complete settings
file. If `CLAUDE_CONFIG_DIR` points elsewhere, install the files there and
adjust the merged hook command paths. Start a new Claude Code session after
changing the configuration.

Claude publishes idle at session start, working when a prompt starts or tool
control returns, waiting when a permission dialog appears, done when the
response stops, and clear when the session ends or switches. Its prompt ID is
the turn identity, preventing a delayed tool event from reopening a completed
prompt. `SessionEnd` covers normal exit, `/clear`, and interactive session
switching.

On Claude Code 2.1.141 or newer, the bridge returns a supported terminal BEL
sequence for a visible permission request and when a final answer stops. A
continuing stop hook does not ring, and ordinary working events do not emit a
bell. This neither approves, denies, nor blocks any Claude action.

## Recovery and cleanup

Both bridges write a server- and pane-scoped host journal before publishing a
small versioned JSON message through Zellij's `vertical-tab-agent-status` pipe.
Sidebar instances also save lifecycle records and exact focus acknowledgements
in the plugin cache.

After detach, session switching, reattach, or plugin reload, a new sidebar
runtime restores the cache and requests a host-journal snapshot. Timestamp,
session, turn, and terminal-completion rules ensure that newer applicable
records win. State is isolated by the live Zellij server process and is not
restored after that server exits and another server starts.

Recovery is best-effort. Denied `RunCommands` permission, a missing helper, or
corrupt cache or journal data does not disable normal live pipes and peer
synchronization. Closing a pane clears its plugin record. Abrupt process failure
cleanup may wait until the pane closes or another agent session starts there.
Outside Zellij, the bridges exit successfully without changing status.

## Adding another agent

Agent integrations are adapters over `hooks/common/agent_bridge.py`. An adapter
maps native hook input into an immutable `AgentUpdate` containing a session ID,
canonical state, canonical lifecycle event, and optional turn ID. The common
runtime owns pane lookup, timestamps, validation, journal-before-pipe ordering,
publication, pruning, and snapshot recovery.

Agent-specific behavior, such as Codex notifier forwarding or Claude Code
`terminalSequence` output, remains in the adapter. Copy `agent_bridge.py` and
`status_store.py` beside every installed adapter so it remains dependency-free.
Adapters must not construct the version-1 wire payload or invoke `zellij pipe`
directly.

See [the development workflow](../DEVELOPMENT.md#agent-adapter-contract) for the
maintainer-facing contract and validation process.
