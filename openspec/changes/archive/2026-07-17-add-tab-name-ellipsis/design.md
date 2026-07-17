## Context

`format_row` currently joins the lead marker, right-aligned tab index, and name into one body string, then clips that string to the available terminal-cell width. When a status badge exists, its suffix is reserved first, but the name still ends abruptly. The implementation already uses `unicode-width` for cell measurement and must continue handling wide characters without splitting them.

The configured `0xProto Nerd Font Mono` owns the Unicode ellipsis glyph `…` with a one-cell advance, so it does not introduce the fallback-font issue previously found in status icons.

## Goals / Non-Goals

**Goals:**

- Indicate omitted name content with a single-cell ellipsis.
- Preserve the lead/index prefix and optional status suffix before allocating cells to the name.
- Keep all rows exactly fitted to the current width.
- Handle ASCII, wide Unicode, and narrow-width edge cases deterministically.

**Non-Goals:**

- Horizontal scrolling, marquee animation, wrapping, or multi-line tab rows.
- Changing the fixed sidebar width.
- Ellipsizing the tab index or status badge.
- Adding a new dependency or configuration option.

## Decisions

### Compose the row from independently budgeted regions

Build the lead/index prefix separately from the tab name. If a badge exists, reserve its width and separating space first. The body formatter then reserves the prefix, gives the remaining cells to the name, and only inserts an ellipsis when the full name exceeds that budget.

This makes overflow explicit and prevents the ellipsis from displacing the status badge. If the width cannot contain the prefix, existing prefix clipping remains the terminal fallback; if a badge alone fills the row, the existing badge-only behavior remains.

### Truncate by terminal cells, not bytes or characters

Reuse `UnicodeWidthChar` to take only complete characters whose cell widths fit before the ellipsis. Pad any unused cell caused by a wide-character boundary after the ellipsis so selected styling still spans the complete row.

Alternatives considered:

- Three ASCII dots consume too much of a 24-column sidebar.
- Blindly replacing the final clipped character can split layout regions or remove the badge separator.
- Character-count truncation mismeasures East Asian wide characters.

## Risks / Trade-offs

- **Very narrow widths cannot show every region:** The prefix or badge can consume all available cells. → Preserve the existing deterministic narrow-row fallbacks and omit the ellipsis when no name cell exists.
- **A wide character may leave one unused budget cell before padding:** → Pad after the ellipsis to maintain exact row width rather than splitting the character.
