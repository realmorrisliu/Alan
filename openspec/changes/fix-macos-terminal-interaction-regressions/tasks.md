## 1. Reproduction And Baseline

- [ ] 1.1 在当前 macOS app 中复现 Vim 快捷键问题，记录被截获的具体按键、focused pane、command input 可见状态和相关日志。
- [x] 1.2 复现新建 split pane 会继承 cwd、New Terminal Tab 不继承 cwd 的差异，记录 runtime metadata cwd 和 `ShellPane.cwd`。
  - 2026-05-17: User confirmed cwd behavior is now OK; this is no longer a live blocker.
- [x] 1.3 复现输入 `exit` 后 pane/tab 未关闭且表现为刷新或 clear 的路径，确认收到的 Ghostty child-exit metadata。
  - 2026-05-17: User confirmed exit behavior is now OK; this is no longer a live blocker.

## 2. Terminal Keyboard Routing

- [x] 2.1 梳理 `TerminalHostView` 的 `performKeyEquivalent`、`keyDown`、command input routing、workspace command routing 和 native command routing 优先级。
- [x] 2.2 调整 focused terminal 的键盘路由，使非 app-reserved 的 Escape、Tab、Backspace、Control/Option 组合键和 Ghostty terminal binding 优先交给 terminal runtime。
- [x] 2.3 修复 IME marked text 场景：组合输入态下 Backspace/Ctrl-H 先进入 `interpretKeyEvents` 更新 preedit，并阻止组合态 control 字符删除已提交 terminal 内容。
- [x] 2.4 保留 command input 可见时的 submit/dismiss/toggle 行为，以及 `Command-T`、`Command-W` 等明确 native workspace shortcut。
- [x] 2.5 为 fake terminal surface 或 focused shell contract 增加键盘输入验证，覆盖 Vim/TUI 关键按键、IME composing Backspace 和 native command shortcut 非回归。
- [x] 2.6 按 Ghostty macOS `SurfaceView_AppKit` 补齐 `performKeyEquivalent`/`doCommand` timestamp redispatch，覆盖 `Control-/`、`Control-Return`、普通 Control/Command key equivalent 和 terminal binding。
- [x] 2.7 补齐 focused terminal 的 local AppKit event monitor：Command keyUp release、active-window focus-only left click suppression、matching mouseDrag/mouseUp suppression。
- [x] 2.8 补齐 modifier event 语义：IME marked text 中不转发 `flagsChanged`，正常路径保留 caps/right-side modifier bits。
- [x] 2.9 增加 focused tests 或 shell contract，证明 Ghostty-style key-equivalent state machine、focus-only click suppression 和 modifier event 语义不会回退。

## 3. New Tab Cwd Inheritance

- [x] 3.1 在 `ShellHostController` 中增加 focused-pane cwd 解析路径，优先使用 runtime metadata `workingDirectory`，再 fallback 到 `ShellPane.cwd`。
- [x] 3.2 让 user/menu/keyboard 发起的 New Terminal Tab 使用 resolved focused cwd；保留 control-plane 显式 cwd 覆盖语义。
- [x] 3.3 确认 New alan Tab 是否应共享同一 cwd 解析 helper；本次只改 New Terminal Tab，New alan Tab 保持原行为。
- [x] 3.4 增加 focused model/controller 测试，覆盖 runtime cwd、snapshot cwd、explicit cwd 和 default/home fallback。

## 4. Child Exit Lifecycle

- [x] 4.1 将 Ghostty child-exit metadata 或 runtime snapshot 更新接到 shell controller 的 pane lifecycle 处理路径。
- [x] 4.2 实现 split pane 中 `exit` 后关闭 owning pane，且不影响 sibling pane runtime identity。
- [x] 4.3 实现 single-pane tab 中 `exit` 后关闭 owning tab，并把 focus 移到下一个有效 tab 或 empty-space state。
- [x] 4.4 实现 final-pane 安全行为：关闭 final pane 后保留 focused empty Space，并避免自动重启 runtime。
- [x] 4.5 增加 text delivery after exit 的失败验证，确保 exited runtime 不再报告 delivery success。
- [x] 4.6 参考 Ghostty macOS 的 close notification/controller 模型，新增 surface close-request 通道，让 non-confirming close request 直接关闭 owning pane/tab。

## 5. Verification And Handoff

- [x] 5.1 运行并更新相关 Swift focused scripts 与 `clients/apple/scripts/check-shell-contracts.sh`。
- [ ] 5.2 构建并运行 macOS app，手工验证 Vim/nvim 的插入、退出、移动、保存/退出等快捷键路径。
- [x] 5.3 手工验证同一 cwd 下 split pane 与 New Terminal Tab 的 cwd 一致性。
  - 2026-05-17: User confirmed cwd behavior is OK.
- [x] 5.4 手工验证 split pane、single-pane tab 和 final-pane/fallback 的 `exit` 行为。
  - 2026-05-17: User confirmed exit behavior is OK.
- [x] 5.5 更新实现说明或 verification notes，列出覆盖的按键、cwd 路径、exit 场景和任何剩余限制。
- [ ] 5.6 实现合入后，将 delta specs 同步到 `openspec/specs/` 并准备 archive。
