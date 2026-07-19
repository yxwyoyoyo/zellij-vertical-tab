## 1. Native notification path

- [x] 1.1 Configure Codex completed-turn and approval-request notifications to always emit BEL so Zellij can handle active and inactive tabs
- [x] 1.2 Map Zellij's persistent tab bell state into the sidebar row model
- [x] 1.3 Render a distinct theme-colored attention icon without obscuring pane-specific lifecycle status

## 2. Width and styling behavior

- [x] 2.1 Reserve status and attention suffixes before truncating names
- [x] 2.2 Prioritize the attention icon when an extremely narrow row cannot show both suffixes
- [x] 2.3 Preserve native selected-row styling and independent semantic colors

## 3. Validation and delivery

- [x] 3.1 Add unit tests for one-pane, multi-pane, combined-suffix, and narrow-width behavior
- [x] 3.2 Update README and OpenWiki documentation
- [x] 3.3 Run the complete local gate and strict OpenSpec validation
- [x] 3.4 Build, install, and hot-reload the release artifact in the active Zellij session
- [x] 3.5 Verify a direct BEL sets persistent attention in the active Zellij session, validate the corrected `always` Codex configuration, and receive user acceptance
