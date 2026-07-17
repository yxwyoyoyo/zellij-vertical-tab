## Context

The plugin renders status suffixes through Zellij's `Text` component. iTerm is configured with `0xProto Nerd Font Mono`, but the previous geometric Unicode symbols are absent from that font. HarfBuzz resolves them as `.notdef`, causing macOS to substitute another font with different ascent and descent metrics. Zellij exposes row-level terminal coordinates rather than sub-cell vertical positioning, so offsetting a fallback glyph is neither available nor portable.

The `Text` component supports theme-relative emphasis, dim, and semantic success styles. These styles apply by character index and retain the selected text background.

## Goals / Non-Goals

**Goals:**

- Render all status icons from the configured Nerd Font with identical vertical metrics and one-cell advances.
- Distinguish states with theme-aware colors without emitting hard-coded ANSI RGB sequences.
- Color the complete aggregate badge, including its pane count, while preserving selected-row styling.
- Verify glyph width, state-to-style mapping, WASM compatibility, and the installed release.

**Non-Goals:**

- Detect or install a terminal font at runtime.
- Add configuration for alternate icon packs or colors.
- Change lifecycle transport, aggregation precedence, or session cleanup behavior.
- Persist plugin state across hot reloads.

## Decisions

### Use native Font Awesome icons from Nerd Font Mono

Map idle, working, waiting, and done to ``, ``, ``, and ``. In `0xProtoNFM` these glyphs resolve natively, advance by one terminal cell, and share the same vertical bounding box. This removes fallback-font baseline drift while retaining recognizable icons.

Alternatives considered:

- Geometric Unicode symbols remain portable in text but are missing from the active font and therefore misalign.
- ASCII labels share the text baseline but do not satisfy the icon requirement.
- Manual vertical offsets are unavailable at sub-cell resolution in Zellij's text API.

### Use Zellij semantic text styles

Color idle with the dim style, working with text emphasis level 1, waiting with text emphasis level 0, and done with the semantic success style. The default theme maps these to dim, cyan, orange, and green, while custom themes remain authoritative.

Apply styling to the badge's final character range after width fitting. This includes a multi-pane count and allows the same indices to work when the row is selected.

### Keep color logic separate from state precedence

The existing aggregation continues to select the dominant state. A small state-to-style mapping controls presentation only, making it independently testable and preventing rendering concerns from changing lifecycle semantics.

## Risks / Trade-offs

- **Nerd Font dependency:** Other fonts may show missing or substituted icons. → Document the requirement and the tested `0xProto Nerd Font Mono` profile.
- **Theme variation:** Custom themes may map emphasis colors differently from the default cyan/orange palette. → Describe colors as semantic theme roles and avoid hard-coded RGB values.
- **Hot reload clears status:** Replacing the WASM resets in-memory records. → Republish live session status after development reloads; persistence remains outside this change.
