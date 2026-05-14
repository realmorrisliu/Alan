# alan for macOS Terminal Workspace UI / UX Contract

> Status: superseded bridge. The canonical current macOS terminal workspace UI/UX contract
> lives in OpenSpec long-lived specs under `openspec/specs/`.

## Purpose

This file remains only as a stable pointer for older links. The previous
narrative contract in this file described an earlier sidebar and multi-section
command direction and is no longer the source of truth.

Use these OpenSpec capabilities instead:

1. `openspec/specs/macos-shell-ui-ux-conformance/spec.md`
2. `openspec/specs/macos-shell-workspace-interactions/spec.md`
3. `openspec/specs/macos-shell-build-test-contract/spec.md`
4. `openspec/specs/macos-shell-terminal-lifecycle/spec.md`
5. `openspec/specs/macos-shell-control-plane-reliability/spec.md`
6. `openspec/specs/macos-terminal-runtime-foundation/spec.md`
7. `openspec/specs/macos-terminal-surface-parity/spec.md`
8. `openspec/specs/macos-app-instance-lifecycle/spec.md`
9. `openspec/specs/macos-app-architecture-maintainability/spec.md`

## Current Direction

The current UI contract is terminal-first and light-mode-first:

1. the active terminal tab remains the center of gravity,
2. spaces and tabs live in a single material sidebar column,
3. spaces switch through a compact bottom switcher,
4. `Command-P` opens a single floating Liquid Glass-style command input,
5. default command input does not show candidate rows or multi-section command
   surfaces,
6. inspector chrome is not part of the default shell,
7. raw IDs, bindings, runtime phases, and diagnostics remain out of default UI.

When this bridge conflicts with OpenSpec, OpenSpec wins.
