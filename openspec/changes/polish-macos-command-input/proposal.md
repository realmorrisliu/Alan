## Why

`Command-K` currently opens a large command palette with action, routing, and
attention candidate sections. That makes the shortcut feel like a dashboard
surface instead of a focused, high-quality macOS command input. Alan should make
`Command-K` a beautiful Liquid Glass-style input layer that is quick to summon,
easy to dismiss, and visually consistent with the material polish direction.

## What Changes

- Redesign `Command-K` as a single floating input field with Liquid Glass-style
  material, subtle depth, and native focus behavior.
- Remove the always-visible candidate/action/routing/attention lists below the
  input.
- Keep the input field centered on typing and submission; typed command routing
  may execute known commands, but suggestions are not shown in the default UI.
- Remove command-palette voice/microphone affordances from this surface unless a
  future voice-specific change reintroduces them in its own interaction model.
- Preserve keyboard behavior: `Command-K` opens/focuses the input, Return submits
  when a typed command can be resolved, Escape/click-away dismisses, and terminal
  focus returns after dismissal.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: add a requirement that `Command-K` is a
  floating Liquid Glass-style input rather than a multi-section command palette.
- `macos-shell-workspace-interactions`: clarify command UI routing without
  default visible candidate lists.
- `macos-shell-build-test-contract`: require focused keyboard, visual, and focus
  verification for the command input.

## Impact

- Affected Apple client code:
  - `clients/apple/AlanNative/MacShellRootView.swift`
  - `clients/apple/AlanNative/Views/Shell/ShellCommandTabView.swift`
  - `clients/apple/AlanNative/Views/Shell/ShellSidebarView.swift`
  - `clients/apple/AlanNative/Support/ShellDesignTokens.swift`
  - `clients/apple/scripts/check-shell-contracts.sh`
- No daemon, runtime, terminal protocol, or shell control-plane command changes.
- This may delete or heavily simplify `ShellCommandRow`,
  `ShellCommandTabIntent`, and candidate-section rendering from the default
  command surface.
- Research inputs:
  - Apple Human Interface Guidelines, Materials:
    `https://developer.apple.com/design/human-interface-guidelines/materials`
  - Apple Human Interface Guidelines, Search fields:
    `https://developer.apple.com/design/human-interface-guidelines/search-fields`
  - Apple Technology Overview, Liquid Glass:
    `https://developer.apple.com/documentation/TechnologyOverviews/liquid-glass`
