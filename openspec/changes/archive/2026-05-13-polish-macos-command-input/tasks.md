## 1. Command Surface Simplification

- [x] 1.1 Replace the default `ShellCommandTabView` body with an input-only floating command surface.
- [x] 1.2 Remove default rendering for action, routing, attention, best-match, command-row, and microphone affordances from the command input.
- [x] 1.3 Keep `Command-P` open/focus, Escape, click-away, close, and focus-restoration behavior intact.

## 2. Typed Command Routing

- [x] 2.1 Preserve a small deterministic typed-command resolver for existing workspace actions that are safe to execute from Return.
- [x] 2.2 Route resolved commands through the same `ShellHostController` workspace command handlers used by menus and shortcuts.
- [x] 2.3 Show unresolved command state inline without opening candidate rows or exposing raw debug identifiers.

## 3. Material And Verification

- [x] 3.1 Apply the shared floating/liquid material role to the command input surface.
- [x] 3.2 Run focused Apple build or shell UI checks affected by command input changes.
- [x] 3.3 Capture screenshot or manual review notes for the default light-mode command input.
- [x] 3.4 Run `openspec validate --all --strict` before PR.

### Verification Notes

- 2026-05-12: Chose `Command-P` for the command input shortcut. It intentionally replaces the app-level Print shortcut for Alan's shell window, avoids terminal `Command-K` clear/edit conflicts, and keeps the visible hint compact.
- 2026-05-12: Built `AlanNative` Debug for macOS after the command input changes. Xcode still reported the local CoreSimulator version warning, but the macOS build succeeded.
- 2026-05-12: Ran `bash clients/apple/scripts/check-shell-contracts.sh`; shell contract checks passed.
- 2026-05-12: Ran `openspec validate --all --strict`; all 25 items passed.
- 2026-05-12: Captured default `Command-P` command input screenshot at `/tmp/alan-ui-polish-command-input-command-p-retry.png`; the surface is a single floating input with no action, routing, attention, best-match, command-row, or microphone affordances.
- 2026-05-12: Captured unresolved Return state at `/tmp/alan-ui-polish-command-input-unresolved-return-retry.png`; unresolved text stays input-only and shows `No matching command.` inline.
- 2026-05-12: Captured resolved typed command state at `/tmp/alan-ui-polish-command-input-resolved-new-tab.png`; submitting `new tab` used the shared workspace command path, dismissed the input, and created a new terminal tab.
- 2026-05-12: Captured Escape/focus-restoration state at `/tmp/alan-ui-polish-command-input-escape-focus.png`; Escape dismissed the input and returned focus to the terminal prompt.
