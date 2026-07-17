## 1. Row Formatting

- [x] 1.1 Split row composition into prefix, name, and optional status-suffix budgets
- [x] 1.2 Add terminal-cell-aware name truncation with a single-cell ellipsis
- [x] 1.3 Preserve deterministic behavior when the prefix or badge leaves no name cells

## 2. Verification and Documentation

- [x] 2.1 Add unit tests for fitting names, ASCII overflow, wide-character overflow, badges, and narrow rows
- [x] 2.2 Document ellipsized overflow behavior and update the rendered example
- [x] 2.3 Run formatting, host tests, WASM Clippy, release build, and strict OpenSpec validation
- [x] 2.4 Install and hot-reload the release WASM, then restore live agent statuses
