## Automated Verification

- `bash clients/apple/scripts/test-terminal-surface-controller.sh`
  - 覆盖 Escape、Tab、Control-W、Option-F 作为 terminal-owned key 的 routing。
  - 覆盖 `Command-T` 仍然走 native New Terminal Tab shortcut。
- `bash clients/apple/scripts/test-shell-runtime-metadata.sh`
  - 覆盖 New Terminal Tab 继承 focused runtime cwd。
  - 覆盖 snapshot cwd fallback。
  - 覆盖 explicit cwd override。
  - 覆盖 split pane child exit、single-pane tab child exit、final pane child exit into empty focused Space。
- `bash clients/apple/scripts/test-terminal-runtime-service.sh`
  - 覆盖 exited runtime 的 text delivery 使用 `terminal_child_exited` 失败码，且不会把 text 送到 surface。
- `bash clients/apple/scripts/check-shell-contracts.sh`
  - 覆盖 shell contract 静态检查。
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath /private/tmp/alan-derived-data build`
  - Debug build succeeded. The first build attempt without `-derivedDataPath` failed because the sandbox could not write `~/Library/Developer/Xcode/DerivedData`.

## Manual Verification Pending

用户将自行验证当前运行中的 alan app：

- Vim/nvim 中 Escape、Tab、Backspace、Control-W、Control-F、Control-B、Option-modified navigation 是否正常进入 terminal。
- 在 pane 中 `cd` 后新建 Terminal Tab 是否继承当前 cwd。
- split pane、single-pane tab、final pane 中输入 `exit` 后是否关闭 owning pane/tab，且不会 clear/restart 成新 terminal。

## Implementation Boundary

本次只改变 New Terminal Tab 的 cwd 继承。New alan Tab 暂时保持原行为，避免把 terminal bugfix 扩大为 agent tab 启动语义变更。
