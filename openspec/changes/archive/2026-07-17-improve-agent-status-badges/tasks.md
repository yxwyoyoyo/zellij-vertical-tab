## 1. Badge Presentation

- [x] 1.1 Replace fallback Unicode symbols with native single-cell Nerd Font icons for every renderable agent state
- [x] 1.2 Map idle, working, waiting, and done to distinct Zellij theme styles
- [x] 1.3 Apply state styling to the complete badge suffix while retaining active-row selected styling

## 2. Tests and Documentation

- [x] 2.1 Add host regression tests for icon cell width and state-to-theme-style mapping
- [x] 2.2 Update examples, badge meanings, colors, and the Nerd Font requirement in the README

## 3. Verification and Activation

- [x] 3.1 Verify the configured terminal font owns the selected icons and gives them identical vertical metrics
- [x] 3.2 Run formatting, host tests, WASM Clippy, release build, and strict OpenSpec validation
- [x] 3.3 Install and hot-reload the release WASM, then restore live session badges after reload
