## Context

`wait_for_process_exit` discovers `select.kqueue` dynamically. macOS uses the event-driven path; Linux lacks the kqueue symbols and uses the polling path. The tests currently patch individual attributes on the real `select` module, which makes their setup depend on the host platform before the function runs.

## Goals / Non-Goals

**Goals:**

- Exercise both watcher branches on macOS and Linux.
- Keep assertions over event registration, queue control/closure, polling, and sleep behavior.
- Preserve the current runtime implementation.

**Non-Goals:**

- Add Windows support; the bridge continues to require Unix `fcntl`.
- Replace polling with a Linux-specific process API.
- Introduce containers or hosted CI.

## Decisions

### Mock the module dependency instead of host attributes

Each test replaces `AGENT_STATUS.select` with a mock object. The kqueue test defines the factory, event constructor, and constants on that mock; the polling test sets `kqueue` to `None`. This makes capability discovery deterministic without asking `patch.object` to find attributes that Linux does not provide.

Adding `create=True` separately to every macOS attribute was rejected because it remains coupled to the concrete module shape and is easier to make incomplete when the runtime reads another constant.

### Keep runtime and Unix boundary unchanged

The production code already uses `getattr(select, "kqueue", None)` and falls back to polling. Only tests and documentation change. `fcntl` remains a supported Unix assumption rather than being hidden behind test mocks.

## Risks / Trade-offs

- **A broad module mock could make assertions too permissive** → set the exact kqueue values used by the runtime and assert the full `kevent` call.
- **Local macOS success does not prove Linux's concrete module shape** → make the polling test's replacement dependency explicitly omit kqueue capability and avoid touching the host module altogether.
