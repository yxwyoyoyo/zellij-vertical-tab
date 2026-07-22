## 1. Layout change

- [x] 1.1 Replace `size="13%"` with `size=32` in `zellij.kdl` dev layout

## 2. Spec update

- [x] 2.1 Update the layout-integration requirement and scenarios to describe a fixed rather than percentage-based width

## 3. Verification

- [x] 3.1 Run `mise run check` as the full pre-merge gate
- [x] 3.2 Start a fresh Zellij session and confirm the sidebar renders at 32 columns, resize drag works, and new tabs get the same fixed width
