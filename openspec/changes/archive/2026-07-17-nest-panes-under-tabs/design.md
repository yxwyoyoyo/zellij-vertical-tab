## Context

The sidebar currently reduces `PaneManifest` to `pane_id -> tab_position` and renders exactly one row per `TabInfo`. Agent records are already keyed by terminal pane ID, but rendering aggregates them into a single tab badge. Zellij 0.44.3 supplies a flat pane list per tab with stable IDs, titles, focus and layer flags, and geometry; it does not expose the original split tree.

The plugin still runs once per tab through `default_tab_template`, so the existing peer synchronization, delayed `set_selectable(false)`, WASI binary target, and layout safety constraints remain in force.

## Goals / Non-Goals

**Goals:**

- Show per-pane identity and agent state when a tab has multiple terminal panes.
- Preserve one compact row for tabs with zero or one terminal pane.
- Make every rendered row deterministic, width-fitted, scrollable, and clickable.
- Preserve existing status transport, stale-update protection, peer synchronization, and theme-colored icons.

**Non-Goals:**

- Reconstruct Zellij's original recursive split tree from pane geometry.
- Add expand/collapse state, chevrons, or other show/hide controls.
- Display plugin panes such as the sidebar or status bar as children.
- Add agent products or agent sub-session tracking.

## Decisions

### Retain the full terminal-pane manifest

Store terminal `PaneInfo` values grouped by tab position in addition to the existing pane-to-tab ownership map. This provides titles, focus, layer, and geometry for rows while leaving lifecycle cleanup and agent synchronization keyed by stable pane ID.

Alternative: enrich only tracked Codex panes. Rejected because users need pane identity and navigation even when only one of several panes runs an agent.

### Build one flattened row model before rendering and input handling

Create a pure ordered list whose entries are either a tab target or a terminal-pane target. Every tab contributes its parent row. A tab with more than one terminal pane immediately contributes all pane children; a tab with zero or one contributes no child. Rendering, scrolling, overflow indicators, and mouse clicks all consume this same model so their row coordinates cannot diverge.

Alternative: calculate children separately inside rendering and mouse handlers. Rejected because duplicated offset arithmetic would make clicks unreliable after scrolling.

### Use a deterministic visual ordering rather than infer split ancestry

Within a tab, order visible tiled panes by top-to-bottom then left-to-right geometry, followed by floating panes and then suppressed panes, with pane ID as a final stable tie-breaker. Zellij exposes a flat list and overlapping layers, so a deeper tree would be speculative.

### Place badges according to pane cardinality

For zero terminal panes, render no badge. For exactly one terminal pane, retain the compact tab row and render that pane's state on the tab. For multiple terminal panes, omit the parent aggregate and render each pane's own optional state on its child row. This eliminates duplicate or lossy status displays while keeping the common case unchanged.

### Generalize width fitting for tab and pane prefixes

Use one named-row formatter with a caller-provided prefix and optional badge. Tab rows keep the overflow lead and aligned one-based index. Pane rows use the overflow lead plus blank indentation one cell beyond the tab-name column. Every row reserves one trailing padding cell when the available width can still preserve its required prefix or badge. Badge rows reserve the full badge before truncating the title by terminal-cell width, and all long titles append the existing ellipsis when possible. The badge color range excludes the trailing padding cell.

### Dispatch clicks through explicit row targets

A tab target calls `switch_tab_to`. A pane target calls Zellij's `focus_terminal_pane`, which switches to the owning tab/layer and focuses that stable terminal pane. The existing `ChangeApplicationState` permission already covers both actions.

## Risks / Trade-offs

- [Many panes consume vertical space quickly] → Scroll the flattened hierarchy with the existing bounded one-row wheel behavior and overflow indicators.
- [Pane titles can be identical or change dynamically] → Use stable pane IDs for status and click identity; titles are display-only.
- [Floating and suppressed panes overlap tiled geometry] → Sort by explicit layer class before geometry and never claim to represent split ancestry.
- [A focused child and active parent can both look selected] → Apply selected styling to the active tab and focused child; the indentation distinguishes their roles and all other children remain unselected.
- [Pane updates are more frequent and contain more data] → Rebuild a small in-memory row vector only on update/render; no external I/O or new dependency is introduced.

## Migration Plan

1. Install the newly built WASM over the existing configured plugin artifact.
2. Existing one-pane tabs retain their current appearance and behavior immediately.
3. Multi-pane tabs begin showing children as soon as the next `PaneUpdate` arrives.
4. Roll back by restoring the previous WASM; the status bridge protocol and global Codex configuration require no rollback.

## Open Questions

None.
