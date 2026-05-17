## Context

这次 change 处理的是 macOS shell 的 terminal-first 主路径，而不是新增 shell 功能。现状里相关行为分散在几层：

- `TerminalHostView` 先处理 command input / workspace command / native command，再进入 Ghostty key binding 和 `keyDown` 转译。
- `ShellHostController.performShellWorkspaceCommand(.newTerminalTab)` 当前直接调用 `openTerminalTab()`，没有把 focused pane 的运行时 cwd 传入。
- `ShellStateSnapshot.openingTab(...)` 在 `workingDirectory == nil` 时回退到 `defaultShellWorkingDirectory()`，而 split pane 已经从原 pane 继承 cwd。
- Ghostty 的 `GHOSTTY_ACTION_SHOW_CHILD_EXITED` 已经能把 `processExited` 写入 runtime metadata，但 Ghostty macOS app 的正确 close 模型不是 metadata-only：`close_surface_cb` 发出 close request，terminal controller 直接从 surface tree 移除对应 node。alan 之前缺少这条从 embedded surface 到 shell controller 的直接 close-request 通道，导致 `exit` 只能依赖 metadata 旁路。

这三个问题应该作为终端交互回归一起处理，因为它们共享同一个产品边界：终端 host 是活动 pane 的输入和进程生命周期 owner，shell controller 只负责明确的 workspace mutation。

## Goals / Non-Goals

**Goals:**

- Vim、less、fzf、ssh 等 TUI 在 focused terminal pane 中能稳定接收非 app-reserved 的键盘快捷键。
- 新建 terminal tab 默认继承当前 focused pane 的最新 cwd，和 split pane 行为一致。
- shell child 正常退出时关闭对应 pane/tab，或者在不能关闭时进入明确 exited state；不能隐式清屏、重启、刷新成新 shell。
- 给这三个回归补足 focused tests 或手工验证步骤，能在后续 terminal 改动中复用。

**Non-Goals:**

- 不重做 Ghostty 的键盘编码或 pty 协议。
- 不改变全局 macOS app 级快捷键，例如 app quit、显式 command input 切换、菜单声明的 `Command-*` workspace 操作。
- 不恢复旧终端进程；关闭/退出语义只处理当前进程内 runtime 和 shell state。
- 不改变 workspace manifest 的长期持久化格式，除非实现时发现 cwd 继承需要记录新的 manifest 字段。

## Decisions

1. **终端输入优先走 terminal host，再让明确 app command 截获。**

   Focused pane 的 `TerminalHostView` 应该把非 command-key 的控制键、Escape、Tab、Backspace、功能键、Option/Control 组合键和 Ghostty 识别为 terminal binding 的事件交给 terminal surface。IME 组合输入是这个规则的输入法例外：当 AppKit `NSTextInputClient` 已有 marked text 时，Backspace/Ctrl-H 等 control input 必须先交给 `interpretKeyEvents` 更新 preedit，并抑制组合态控制字符漏到 terminal。Command input 可见时仍然优先处理自己的 Escape/Return/Command-P；`Command-T`、`Command-W` 等显式 app/workspace command 继续走 native command routing。

   备选方案是继续先跑 workspace/native routing，再对缺失快捷键逐个开洞。这会把 Vim/TUI 支持变成无穷补丁列表，也容易再次截获 `Ctrl-*` 这类终端快捷键。

2. **新建 Tab 的 cwd 由 shell controller 解析，不下沉到 model 的默认值。**

   `ShellHostController` 应在 user/menu/keyboard/control command 的新建 tab 入口解析 cwd：优先使用 focused pane 的 runtime metadata `workingDirectory`，其次使用 pane snapshot `cwd`，最后才使用 workspace default/home。`ShellStateMutations.openingTab(...)` 继续保留显式 `workingDirectory` 和 default fallback，作为低层 mutation 的安全默认。

   备选方案是在 `openingTab(...)` 内部直接读取 focused pane。这样会让纯 model mutation 同时承担 controller/runtime 语义，不利于单元测试，也会让控制平面显式传入 cwd 的行为更难判断。

3. **child exit 是 lifecycle 事件，不是重启触发器。**

   `GHOSTTY_ACTION_SHOW_CHILD_EXITED`、Ghostty `close_surface_cb` 中 child 已不活跃的 close request，或等价 runtime metadata 应由 `ShellHostController` 观察并转成 pane/tab mutation：split tab 中关闭退出的 pane；单 pane tab 中关闭 tab。实现应参考 Ghostty macOS 的 `ghosttyCloseSurface -> closeSurface(node, withConfirmation:) -> removeSurfaceNode/closeTab/closeWindow` 模型，让 non-confirming close request 直接驱动 shell close path，而不是只写 metadata 等间接观察。如果实现层不能安全关闭最后的可见 shell surface，则保留 exited state 并提供明确的新建/重启入口。任何路径都不能为了保持画面可输入而自动创建新的 shell runtime，也不能在 controller 观察前把 exited metadata 重置回 running。

   备选方案是只在 pane UI 上展示 “exited” 状态。它能解释状态，但不符合用户输入 `exit` 后退出当前 pane/tab 的预期；只适合作为最终 pane 或关闭失败时的 fallback。

4. **验证以 fake runtime 和真实 app 手工清单结合。**

   键盘路由可用 fake surface 捕获 normalized key/text delivery；cwd 和 close mutation 可用 shell model/controller 脚本验证；Vim 的真实快捷键和 macOS responder-chain 行为需要运行 app 手工确认，因为问题本身发生在 AppKit/Ghostty 交界。

5. **Vim/TUI 输入要复用 Ghostty 的 AppKit responder contract，而不是只扩展 `keyDown`。**

   Ghostty macOS 的关键点是 `performKeyEquivalent -> doCommand -> keyDown` 的闭环：Command/Control 组合键会先进入 `performKeyEquivalent`，如果不是 app/menu 已消费的 binding，则记录当前 event timestamp 并让 AppKit 继续；当 AppKit 把同一个事件转成 `doCommand` 时，surface 再把当前 event 送回事件系统，第二次 `performKeyEquivalent` 看到相同 timestamp 后才合成 terminal key event。Alan 需要在 `TerminalHostView` 内保留这套 timestamp state，覆盖 `Control-/ -> Control-_`、`Control-Return`、普通 Control 组合键和 Ghostty terminal binding。`keyDown` 仍负责 IME `interpretKeyEvents`、preedit 同步和最终 `ghostty_surface_key` 发送。

   同一契约还包括两个焦点边界：Command-modified `keyUp` 可能不会走普通 responder chain，所以 focused terminal host 需要用 local event monitor 补发 release；已激活窗口中点击未 focused split pane 时，第一次 left mouse down 只切换 terminal focus，不能同时把 click 注入 Vim mouse mode。Modifier `flagsChanged` 也应和 Ghostty 一样在 marked text 存在时不发 terminal modifier，并保留 caps/right-side modifier bits。

   备选方案是在 `routeKey` 中继续追加 `Ctrl-*` 白名单。这个方案无法覆盖 AppKit 先调用 `performKeyEquivalent`/`doCommand` 的路径，也无法解释为什么 fake adapter 测试通过但真实 Vim 仍失败。

## Risks / Trade-offs

- [Risk] 放宽 terminal input ownership 可能让部分 workspace 快捷键在 terminal focused 时不再触发。→ 保留显式 app-reserved Command shortcuts，并用测试覆盖 command input 和 terminal binding 的优先级。
- [Risk] cwd 继承如果只看 stale pane snapshot，`cd` 后新建 tab 仍可能回到旧目录。→ controller 必须优先读取 runtime metadata，再 fallback 到 `ShellPane.cwd`。
- [Risk] child exit 自动关闭 pane/tab 可能误关正在展示退出信息的任务。→ 只对 shell child exited 的 terminal lifecycle 信号生效；如果关闭会违反 final-pane 保护，则进入明确 exited state。
- [Risk] 真实 Vim 行为很难完全自动化。→ 自动化覆盖 key routing contract，手工验证记录覆盖 `vim`/`nvim` 的实际交互。
- [Risk] local event monitor 可能在多个 pane host 间重复观察事件。→ monitor 只在 hit-tested owning host 或 focused host 上消费事件，并在 teardown 时移除。

## Migration Plan

这是运行时行为修复，不需要数据迁移。实现可以分三步提交：先修 keyboard routing，再修 tab cwd 继承，最后接 child exit lifecycle；每一步都更新 focused tests 或手工验证记录。

## Open Questions

- 最后一个 Pane/Tab 收到 `exit` 时，本次实现选择关闭 final pane 并保留 focused empty Space。现有 shell model 已支持 empty Space，因此不需要保留一个 exited pane 作为默认 fallback。
