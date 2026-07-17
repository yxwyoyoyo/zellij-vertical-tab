## Why

Long tab names are currently cut off without any visual indication that text was omitted. An ellipsis makes overflow immediately recognizable while preserving the fixed-width sidebar and agent-status suffix.

## What Changes

- Append a single-cell ellipsis when a tab name must be truncated.
- Measure truncation in terminal display cells so wide Unicode names remain valid.
- Preserve the tab position prefix and complete right-aligned agent badge before allocating space to the name.
- Keep short names and extremely narrow rows deterministic without unnecessary ellipses.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vertical-tab-sidebar`: Change width-fitted long-name rendering to show a terminal-cell-aware ellipsis while retaining the index and optional status suffix.

## Impact

- `src/main.rs`: row composition and width-fitting helpers plus unit tests.
- `README.md`: overflow behavior and examples.
- No protocol, dependency, layout, or hook changes.
