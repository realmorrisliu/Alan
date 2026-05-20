## 1. Registry Model

- [x] 1.1 Define shell action IDs, target kinds, labels, optional default
  shortcut descriptors, availability results, and execution result types.
- [x] 1.2 Add a shell action registry that can resolve actions for menu bar,
  context-menu, and keyboard surfaces without involving Command UI.
- [x] 1.3 Model current-selection targets separately from context-menu targets
  so right-click actions can operate on a non-selected tab.
- [x] 1.4 Add no-op or disabled behavior for unavailable actions with stable
  diagnostics for focused tests.

## 2. Surface Integration

- [x] 2.1 Route existing shell menu commands through the registry where they
  share shell controller behavior.
- [x] 2.2 Route tab and space context menu items through registry entries while
  preserving the right-click context target.
- [x] 2.3 Route existing shell keyboard shortcuts through registry entries
  without changing their default key equivalents.
- [x] 2.4 Leave `Go to or Command...` behavior unchanged and avoid adding new
  tab/space actions to Command UI in this change.

## 3. Action Coverage

- [x] 3.1 Register existing high-frequency shell actions: new tab, close tab,
  split pane, close pane, focus pane, equalize splits, Find, and Space
  navigation.
- [x] 3.2 Add placeholder-ready registry entries needed by follow-up tab
  organization work: pin, unpin, move tab left/right, and move tab to Space.
- [x] 3.3 Ensure labels and disabled states use user-facing language and avoid
  raw pane IDs, tab IDs, or implementation phases.

## 4. Verification

- [x] 4.1 Add focused tests for action ID uniqueness, target resolution,
  availability, disabled execution, and handler routing.
- [x] 4.2 Add shortcut conflict checks for registered default shortcuts in the
  same shell context.
- [x] 4.3 Add focused menu/context-menu coverage or script checks proving shared
  actions are no longer duplicated outside the registry.
- [x] 4.4 Run relevant Apple shell scripts and the macOS app build command, or
  document any local blocker.
- [x] 4.5 Run `openspec validate add-macos-shell-action-registry --type change --strict --json`.
- [x] 4.6 Run `openspec validate --all --strict --json`.
- [x] 4.7 Run `git diff --check`.

## 5. Archive Readiness

- [x] 5.1 Confirm the registry remains shell-only and does not add Command UI,
  settings, daemon, TUI, or user-custom shortcut scope.
- [x] 5.2 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [x] 5.3 Archive the OpenSpec change after implementation merges.
