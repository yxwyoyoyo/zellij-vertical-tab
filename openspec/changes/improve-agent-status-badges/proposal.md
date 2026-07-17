## Why

The existing agent-status glyphs are not present in the configured terminal font, so iTerm substitutes a fallback font whose ascent and descent do not align with tab names. The monochrome badges also make urgent, active, and completed states harder to distinguish at a glance.

## What Changes

- Use single-cell status icons that are native to Nerd Font Mono builds and share consistent vertical metrics.
- Apply distinct Zellij theme colors to idle, working, waiting, and done badges.
- Preserve the selected tab's full-row styling while coloring only the badge suffix.
- Document the icon-font requirement and state-to-icon/color mapping.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-status`: Define native icon metrics, state-specific theme colors, multi-pane count coloring, and selected-row behavior for rendered badges.

## Impact

- `src/main.rs`: badge glyph selection, text styling, and regression tests.
- `README.md`: rendered examples, icon meanings, colors, and Nerd Font requirement.
- Runtime: requires a Nerd Font-compatible terminal font for the intended icons; transport and lifecycle behavior are unchanged.
