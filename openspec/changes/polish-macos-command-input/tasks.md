## 1. Command Surface Simplification

- [ ] 1.1 Replace the default `ShellCommandTabView` body with an input-only floating command surface.
- [ ] 1.2 Remove default rendering for action, routing, attention, best-match, command-row, and microphone affordances from `Command-K`.
- [ ] 1.3 Keep `Command-K` open/focus, Escape, click-away, close, and focus-restoration behavior intact.

## 2. Typed Command Routing

- [ ] 2.1 Preserve a small deterministic typed-command resolver for existing workspace actions that are safe to execute from Return.
- [ ] 2.2 Route resolved commands through the same `ShellHostController` workspace command handlers used by menus and shortcuts.
- [ ] 2.3 Show unresolved command state inline without opening candidate rows or exposing raw debug identifiers.

## 3. Material And Verification

- [ ] 3.1 Apply the shared floating/liquid material role to the command input surface.
- [ ] 3.2 Run focused Apple build or shell UI checks affected by command input changes.
- [ ] 3.3 Capture screenshot or manual review notes for the default light-mode command input.
- [ ] 3.4 Run `openspec validate --all --strict` before PR.
