## 1. Model And Command Routing

- [ ] 1.1 Add a single global quick-terminal slot that reuses one terminal
  runtime across hide/show and summons it onto the current macOS Space/display.
- [ ] 1.2 Add shared shell command routing for the configurable global toggle
  shortcut, draft default `Option+Space`, plus explicit show, hide, focus,
  close, and promote commands.
- [ ] 1.3 Ensure keyboard shortcuts, menu commands, command input, and supported
  control commands converge on the same shell controller behavior.

## 2. Peak Presentation

- [ ] 2.1 Present the quick terminal through a detached native macOS Peak window
  that does not depend on or raise Alan's main window.
- [ ] 2.2 Keep the Peak composition terminal-first: restrained native material
  chrome, no duplicate sidebar, no inspector, no dashboard header, and no
  floating-card layout.
- [ ] 2.3 Preserve terminal input ownership so `Esc` routes to the terminal
  unless an Alan-owned nested quick-terminal menu or picker is open.
- [ ] 2.4 Avoid focus-loss auto-hide; hide is explicit through toggle or command.

## 3. Runtime Lifecycle And Workspace Promotion

- [ ] 3.1 Preserve quick terminal runtime state across hide/show and tear it down
  only through explicit close semantics.
- [ ] 3.2 Apply quick-terminal cwd creation rules: existing instance cwd,
  focused Alan pane cwd, last quick-terminal cwd, then home.
- [ ] 3.3 Implement `Open in Space` promotion as a move into the selected Alan
  Space/tab that hides the Peak and clears the global quick slot.
- [ ] 3.4 Ensure promotion does not copy the terminal process or keep the same
  runtime visible in both the Peak and the target tab.
- [ ] 3.5 Surface hidden quick-terminal user-actionable activity through the
  existing compact activity and notification policy.

## 4. Verification

- [ ] 4.1 Add focus, display/Space placement, hide/show, close, promote, and
  hidden-activity notification tests.
- [ ] 4.2 Add focused checks for `Esc` terminal routing and focus-loss behavior.
- [ ] 4.3 Run relevant shell model, window, command-routing, and terminal runtime
  tests.
- [ ] 4.4 Run the relevant macOS app build command or document any local blocker.
- [ ] 4.5 Run `openspec validate add-quick-terminal-peak --type change --strict --json`.
- [ ] 4.6 Run `openspec validate --all --strict --json`.
- [ ] 4.7 Run `git diff --check`.

## 5. Archive Readiness

- [ ] 5.1 Review the UI to confirm quick terminal stays terminal-first and does
  not add duplicate sidebar, inspector, or dashboard composition.
- [ ] 5.2 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 5.3 Archive the completed OpenSpec change after implementation merges.
