## 1. Registry Shortcut Descriptors

- [ ] 1.1 Add default shortcut metadata to shell action descriptors.
- [ ] 1.2 Move existing shell shortcuts into registry-backed descriptors without
  changing their key equivalents.
- [ ] 1.3 Add conflict detection for duplicate default shortcuts within the same
  dispatch context.
- [ ] 1.4 Ensure menu shortcut hints are rendered from registry descriptors.

## 2. Keyboard Dispatch

- [ ] 2.1 Route shell keyboard shortcuts through the action registry.
- [ ] 2.2 Resolve keyboard targets from current selected Space, selected Tab, and
  focused pane.
- [ ] 2.3 Apply action availability before invoking handlers.
- [ ] 2.4 Preserve terminal input behavior for keys that belong to the active
  terminal view.
- [ ] 2.5 Preserve Find behavior and Find-owned shortcuts while Find is active.

## 3. Default Coverage

- [ ] 3.1 Preserve existing defaults for new Tab, close Tab, split pane, pane
  focus, equalize panes, and Find where they already exist.
- [ ] 3.2 Add or standardize defaults for next/previous Tab and Move Tab
  Left/Right.
- [ ] 3.3 Add a default for Pin/Unpin current Tab only after Tab organization
  exposes the registry-backed action.
- [ ] 3.4 Add Space navigation defaults for next Space, previous Space, and
  numeric Space selection.
- [ ] 3.5 Keep create/rename/delete Space action-only or menu-only with no
  default shortcut in the first version.

## 4. Verification

- [ ] 4.1 Add tests for default shortcut uniqueness and conflict detection.
- [ ] 4.2 Add tests that existing shortcuts keep their key equivalents.
- [ ] 4.3 Add tests for keyboard target semantics on current selected Tab and
  focused pane.
- [ ] 4.4 Add tests for disabled/unavailable actions showing in menus but not
  mutating state when invoked by shortcut.
- [ ] 4.5 Add tests or script checks for Find-active and terminal-input
  precedence.
- [ ] 4.6 Run relevant Apple shell scripts and the macOS app build command, or
  document any local blocker.
- [ ] 4.7 Run `openspec validate add-macos-shell-keybinding-system --type change --strict --json`.
- [ ] 4.8 Run `openspec validate --all --strict --json`.
- [ ] 4.9 Run `git diff --check`.

## 5. Archive Readiness

- [ ] 5.1 Confirm no user-custom shortcut preference UI, config file, or Command
  UI integration slipped into scope.
- [ ] 5.2 Before archive, sync accepted delta requirements into
  `openspec/specs/`.
- [ ] 5.3 Archive the OpenSpec change after implementation merges.
