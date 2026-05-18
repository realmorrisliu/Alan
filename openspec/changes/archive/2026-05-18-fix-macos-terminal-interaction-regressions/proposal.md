## Why

macOS shell 的终端现在还有几类基础交互回归：Vim 等全屏 TUI 无法稳定收到快捷键，新建 Tab 没有继承当前 Pane 的 cwd，shell 输入 `exit` 后也没有关闭对应 Pane 或 Tab。这些行为直接破坏 terminal-first 的主路径，需要用一个独立 OpenSpec change 追踪到可验证修复。

## What Changes

- 修复终端键盘输入透传，确保 Vim 常用快捷键、控制键、Escape/Tab/Backspace、组合修饰键和 IME 之外的原始按键由终端 host 按 Ghostty macOS 的 AppKit 输入契约路由到 Ghostty/pty，而不是被 SwiftUI、菜单命令、`performKeyEquivalent`/`doCommand` 或 command input 截获。
- 新建 Tab 时继承当前 focused pane 的 cwd，和新建 split pane 的行为保持一致；如果 focused pane 没有有效 cwd，再回退到 workspace default/home。
- 输入 `exit` 或 shell 子进程正常退出时，Pane/Tab 按用户可理解的终端生命周期关闭或进入明确的 exited 状态，而不是刷新成一个看似被 clear 的新终端。
- 补充面向 macOS shell 的自动化/手工验证项，覆盖 Vim 快捷键、Tab cwd 继承、单 pane tab exit、split pane exit 和异常退出状态。

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `macos-shell-terminal-lifecycle`: 明确终端键盘事件归 terminal host 所有、shell child exit 必须驱动 pane/tab lifecycle，而不是隐式刷新或重启 runtime。
- `macos-shell-workspace-interactions`: 明确新建 Tab 的 cwd 继承语义应和 split pane 一致，默认从当前 focused pane 继承。
- `macos-shell-build-test-contract`: 增加 Vim/TUI 输入透传、Tab cwd 继承和 exit 生命周期的验证要求。

## Impact

- Apple client terminal host/runtime: `TerminalHostView.swift`, `TerminalHostRuntime.swift`, `GhosttyLiveHost.swift`, `TerminalRuntimeRegistry.swift`, `TerminalRuntimeService.swift`, `TerminalSurfaceController.swift`。
- Apple client shell model/controller: `ShellHostController.swift`, `ShellStateMutations.swift`, `ShellTreeMutations.swift`, `ShellControlPlane.swift`, `ShellSocketServer.swift`。
- Native command routing: `AlanMacShellCommands.swift`, command input routing, menu/keyboard responder-chain handling。
- Tests/scripts: `clients/apple/scripts/check-shell-contracts.sh` and focused Swift shell/terminal scripts need覆盖这三个回归场景。
