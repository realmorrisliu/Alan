## Why

The completed terminal activity work made progress, command completion, and
agent status visible, but Alan still cannot act on reliable prompt or command
boundaries. Prompt navigation, copy-last-output, and command-output search need
their own implementation change so accepted activity specs do not claim
unimplemented semantic terminal behavior.

## What Changes

- Add pane-scoped semantic command metadata for prompt marks, command ranges,
  output ranges, command text, cwd, exit status, timestamps, and reliability.
- Expose command-aware actions through `Go to or Command...` only when reliable
  boundaries exist: jump previous prompt, jump next prompt, copy last command
  output, and search last command output.
- Fall back to ordinary scrollback search, terminal selection/copy, and normal
  scrollback navigation when reliable boundaries are unavailable.
- Keep the MVP action-only: no command browser, no visible command blocks, no
  persistent output segmentation, and no long-term command history database.
- Reuse pane-scoped terminal search ownership for search-last-output and
  fallback scrollback search.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-terminal-activity-semantics`: Adds semantic command-boundary metadata
  as pane-scoped terminal semantics that can feed command-aware actions.
- `macos-terminal-surface-parity`: Adds pane-scoped copy, prompt navigation,
  and command-output search behavior backed by reliable command boundaries.

## Impact

- Apple terminal runtime and shell integration paths that can observe prompt
  marks, command start/end, command text, cwd, exit status, and output ranges.
- `TerminalRuntimeService`, `TerminalHostRuntime`, terminal surface state, and
  pane metadata projection.
- Command input / `Go to or Command...` actions for focused terminal panes.
- Terminal search and clipboard paths for command-output scoped operations.
- Focused Swift/script coverage and OpenSpec validation.
