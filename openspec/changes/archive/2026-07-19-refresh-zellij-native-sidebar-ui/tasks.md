## 1. Native hierarchy renderer

- [x] 1.1 Replace per-row `Text` drawing with one native Zellij nested-list component while preserving flattened row order and selection
- [x] 1.2 Fit tab and pane content inside native top-level and child chrome budgets while preserving overflow markers, ellipsis, right inset, and badges
- [x] 1.3 Apply the existing state-specific badge styles through native nested-list item ranges
- [x] 1.4 Remove displayed tab indices so the native `>` bulletin is the sole leading tab marker without changing internal ordering or targets

## 2. Test coverage

- [x] 2.1 Update row-format tests for native content budgets, hierarchy levels, narrow widths, wide characters, and badge preservation
- [x] 2.2 Add renderer-model tests proving selected rows and item indentation map correctly without changing click targets
- [x] 2.3 Update formatting tests for index-free tab labels, overflow leads, narrow widths, ellipsis, and badge preservation

## 3. Runtime and documentation

- [x] 3.1 Build and hot-reload the WASM, then verify active/inactive tabs, multi-pane children, all badge states, overflow, and narrow/wide widths
- [x] 3.2 Update README, development guidance, and OpenWiki for the native Zellij list presentation
- [x] 3.3 Run `mise run check`, strict OpenSpec validation, and diff hygiene
- [x] 3.4 Update documentation, verify the index-free native rows at runtime, and rebuild/install the release
