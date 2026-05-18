## Automated Verification

- `bash clients/apple/scripts/test-terminal-surface-controller.sh`
  - 覆盖 printable physical keys（包括 `a` 和 Vim command-mode `:`）先进入 AppKit text interpretation，让 IME composition 可以启动；HostView 最终仍应把 committed text 重新包成 Ghostty key event，而不是 programmatic text injection。
  - 覆盖 Escape、Tab、Control-W、Option-F 作为 terminal-owned key 的 routing。
  - 覆盖 `Command-T` 仍然走 native New Terminal Tab shortcut。
  - 覆盖 Ghostty-style `performKeyEquivalent`/`doCommand` timestamp redispatch：terminal binding 直接进入 terminal，`Control-/` 归一化为 `Control-_`，普通 Command/Control key equivalent 先让 AppKit responder chain 处理，再在同一 timestamp 重新派发时只送入 terminal 一次。
  - 覆盖 active/key window 中点击未 focused terminal split pane 的 focus-only 行为，以及 matching left mouse down / drag / mouse up 的 suppression；即使 AppKit 在 local monitor 判定 focus-only 后仍派发 native mouseDown，也不会触发 terminal selection press。
  - 覆盖 `AlanTerminalInputRouter` 是 focus-only primary button sequence、normal-buffer selection drag、alternate-screen/mouse-reporting delivery 和 surface readiness pointer policy 的单一 owner；`TerminalHostView` 只执行 router decision。
  - 覆盖 Backspace 在非组合态仍是 terminal-owned key，但 IME marked text 存在时会进入 `NSTextInputClient`/`interpretKeyEvents`，并抑制组合态 Backspace/Ctrl-H control 字符漏到 terminal。
  - 覆盖 surface handle 的 close request 会被 `TerminalSurfaceController` 转发给 shell owner。
- `bash clients/apple/scripts/test-shell-runtime-metadata.sh`
  - 覆盖 New Terminal Tab 继承 focused runtime cwd。
  - 覆盖 snapshot cwd fallback。
  - 覆盖 explicit cwd override。
  - 覆盖 split pane child exit、single-pane tab child exit、final pane child exit into empty focused Space。
- `bash clients/apple/scripts/test-terminal-runtime-service.sh`
  - 覆盖 exited runtime 的 text delivery 使用 `terminal_child_exited` 失败码，且不会把 text 送到 surface。
- `bash clients/apple/scripts/check-shell-contracts.sh`
  - 覆盖 shell contract 静态检查，包括 `performKeyEquivalent`、`doCommand`、local AppKit event monitor、`AlanTerminalInputRouter` 存在，以及 `TerminalHostView` 不再持有 focus-click suppression state。
  - 覆盖 `TerminalHostView.keyDown` 不调用 programmatic text injection，物理键盘输入保持在 Ghostty key event path。
  - 覆盖 focus-only mouse routing 使用 `terminalInputIsActive`，也就是 shell selection 和 AppKit first-responder 同时成立，而不是只看 raw first-responder state。
  - 覆盖 terminal input trace 的 user-defaults 开关会运行中 refresh，避免打开或关闭诊断日志必须重启 alan。
  - 覆盖 GhosttyKit modulemap contract：prepared local artifacts 不能继续使用 `umbrella header "ghostty.h"`，避免 Clang 对 `ghostty/vt/*` internal headers 产生 umbrella-header warnings。
- `openspec validate fix-macos-terminal-interaction-regressions --strict`
  - Strict validation succeeded after adding the terminal input router requirements and build-test contract coverage.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath /private/tmp/alan-derived-data build`
  - Debug build succeeded. The first build attempt without `-derivedDataPath` failed because the sandbox could not write `~/Library/Developer/Xcode/DerivedData`.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination 'platform=macOS' -derivedDataPath /private/tmp/alan-macos-derived-data build`
  - Debug build succeeded after the close-surface callback fix and Ghostty AppKit responder-contract implementation. Xcode still printed CoreSimulator/log-permission noise, but the macOS target completed with `BUILD SUCCEEDED`.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath /private/tmp/alan-xcode-derived-input-router build`
  - Debug build succeeded after the terminal input router refactor. Xcode still printed the existing CoreSimulator/log-permission noise and GhosttyKit umbrella-header warnings, but the macOS target completed with `BUILD SUCCEEDED`.
- `bash clients/apple/scripts/setup-local-ghosttykit.sh`
  - Refreshed local GhosttyKit artifacts and normalized all generated module maps to `header "ghostty.h"`.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS,arch=arm64 -derivedDataPath /private/tmp/alan-xcode-derived-ghosttykit-modulemap build`
  - Debug build succeeded after the GhosttyKit modulemap normalization.
  - The previous `GhosttyKit` umbrella-header warnings did not reappear.
  - The narrower destination removed the previous "Using the first of multiple matching destinations" warning.
  - Xcode still printed CoreSimulator/cache/log-permission warnings from the local toolchain environment; those are not emitted by alan source or GhosttyKit module maps.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination generic/platform=macOS -derivedDataPath /private/tmp/alan-xcode-derived-generic-macos build`
  - Debug build succeeded with a generic macOS destination, preserving the universal arm64/x86_64 build while avoiding the previous multiple matching destinations warning.
  - The previous `GhosttyKit` umbrella-header warnings did not reappear.
  - Xcode still printed CoreSimulator/cache/log-permission warnings from the local toolchain environment; simulator device support was not required for this macOS build.

## Manual Verification

2026-05-18: 用户确认当前运行中的 alan app 里 Vim/nvim 路径已经 OK。

- 手工 Vim smoke 具体步骤：
  - 在 focused Alan terminal pane 中运行 `vim /tmp/alan-vim-input-smoke.txt`。
  - 输入 `iabc` 应进入 insert mode 并写入文本。
  - 按 `Esc` 应离开 insert mode。
  - 按 `:` 应打开 Vim command mode，而不是把冒号插入文件。
  - 输入 `:q!` 应退出 Vim。
  - 在 Vim 中输入 `iabc` 后按 `Control-[`，应像 Escape 一样离开 insert mode。
  - 在 Vim/nvim 中测试 arrow keys、`Control-W`、Tab 和 Backspace；这些按键应送达 focused terminal pane，而不是触发 alan workspace command。
- Vim/nvim 中 Escape、Tab、Backspace、Control-W、Control-F、Control-B、Control-]、Control-/, Control-Return、Command-modified terminal binding 和 Option-modified navigation 是否正常进入 terminal。
- 中文/日文/韩文输入法组合输入中，Backspace 是否删除 marked text，而不是删除已提交 terminal 内容或无响应。
- 同一窗口中从未 focused split pane 切换 focus 时，第一次 click/drag 是否只聚焦、不注入 Vim mouse mode 或触发 terminal selection；窗口未激活时第一次 click 是否仍能按系统语义激活窗口。
- 在 pane 中 `cd` 后新建 Terminal Tab 是否继承当前 cwd。
- split pane、single-pane tab、final pane 中输入 `exit` 后是否关闭 owning pane/tab，且不会 clear/restart 成新 terminal。
- 如果旧运行中 app 仍表现为 clear/restart，重启到包含 Ghostty-style close-request 通道和 close-surface callback fix 的 build 后再验证；旧 build 只处理了 `SHOW_CHILD_EXITED`/runtime metadata 路径，漏掉了真实 close request 观测路径。

## Implementation Boundary

本次只改变 New Terminal Tab 的 cwd 继承。New alan Tab 暂时保持原行为，避免把 terminal bugfix 扩大为 agent tab 启动语义变更。
