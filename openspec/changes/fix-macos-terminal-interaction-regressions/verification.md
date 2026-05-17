## Automated Verification

- `bash clients/apple/scripts/test-terminal-surface-controller.sh`
  - 覆盖 Escape、Tab、Control-W、Option-F 作为 terminal-owned key 的 routing。
  - 覆盖 `Command-T` 仍然走 native New Terminal Tab shortcut。
  - 覆盖 Ghostty-style `performKeyEquivalent`/`doCommand` timestamp redispatch：terminal binding 直接进入 terminal，`Control-/` 归一化为 `Control-_`，普通 Command/Control key equivalent 先让 AppKit responder chain 处理，再在同一 timestamp 重新派发时只送入 terminal 一次。
  - 覆盖 active/key window 中点击未 focused terminal split pane 的 focus-only 行为，以及 matching left mouse up 的一次性 suppression。
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
  - 覆盖 shell contract 静态检查，包括 `performKeyEquivalent`、`doCommand`、local AppKit event monitor 和 focus-only mouse-up suppression 结构。
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath /private/tmp/alan-derived-data build`
  - Debug build succeeded. The first build attempt without `-derivedDataPath` failed because the sandbox could not write `~/Library/Developer/Xcode/DerivedData`.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination 'platform=macOS' -derivedDataPath /private/tmp/alan-macos-derived-data build`
  - Debug build succeeded after the close-surface callback fix and Ghostty AppKit responder-contract implementation. Xcode still printed CoreSimulator/log-permission noise, but the macOS target completed with `BUILD SUCCEEDED`.

## Manual Verification Pending

用户将自行验证当前运行中的 alan app：

- Vim/nvim 中 Escape、Tab、Backspace、Control-W、Control-F、Control-B、Control-]、Control-/, Control-Return、Command-modified terminal binding 和 Option-modified navigation 是否正常进入 terminal。
- 中文/日文/韩文输入法组合输入中，Backspace 是否删除 marked text，而不是删除已提交 terminal 内容或无响应。
- 同一窗口中从未 focused split pane 切换 focus 时，第一次 click 是否只聚焦、不注入 Vim mouse mode；窗口未激活时第一次 click 是否仍能按系统语义激活窗口。
- 在 pane 中 `cd` 后新建 Terminal Tab 是否继承当前 cwd。
- split pane、single-pane tab、final pane 中输入 `exit` 后是否关闭 owning pane/tab，且不会 clear/restart 成新 terminal。
- 如果旧运行中 app 仍表现为 clear/restart，重启到包含 Ghostty-style close-request 通道和 close-surface callback fix 的 build 后再验证；旧 build 只处理了 `SHOW_CHILD_EXITED`/runtime metadata 路径，漏掉了真实 close request 观测路径。

## Implementation Boundary

本次只改变 New Terminal Tab 的 cwd 继承。New alan Tab 暂时保持原行为，避免把 terminal bugfix 扩大为 agent tab 启动语义变更。
