## Why

alan 的 macOS shell 现在有 `shell-state-window_main.json` 形式的运行快照持久化，但主窗口启动仍然走 fresh bootstrap，且现有模型没有表达 Space 永久存在、Pinned Tab、未 pin Tab 的 12 小时生命周期，导致 App 退出再打开后用户组织好的 Space 和 Tab 不会作为产品状态恢复。

这个 change 将 workspace 持久化从运行态快照中拆出来：Space / Tab 的长期意图由一个版本化 manifest 表达，terminal runtime 仍然由当前进程内 runtime service 管理。

## What Changes

- 新增 `ShellWorkspaceManifest` 作为 macOS shell workspace 的持久化权威，存储 Space、Tab、最后选择状态、pin 快照、TTL 元数据和 active-task 摘要。
- 启动时从 manifest materialize 当前 `ShellStateSnapshot`；manifest 缺失时创建默认 workspace，manifest 损坏时旁路坏文件并创建默认 workspace。
- Space 成为长期对象：创建后一直存在，自动回收 Tab 或关闭最后一个 Tab 不会删除 Space；只有显式删除 Space 才移除。
- 引入 Pinned Tab 快照语义：pin 时是单 pane 就保存 cwd；pin 时是 split 就保存当时 split layout 和每个 pane cwd；pin 后的临时 split/cwd 改动不会自动更新 pin 快照。
- 引入 Unpinned Tab 生命周期：未 pin Tab 跨 App 重启恢复显示，直到超过 12 小时且没有 active task 后自动回收；恢复是新 terminal/runtime，不是旧进程恢复。
- 增加 terminal-aware active-task 信号：前台命令或 alan session active/pending 阻止回收，idle shell 不阻止回收；`processExited == false` 不能单独代表 active task。
- 保持存储后端为 Application Support 下的 versioned Codable JSON，不引入 SwiftData、Core Data 或 SQLite。
- 不迁移旧 `shell-state-window_main.json` 到 manifest；首次没有 manifest 时创建 fresh default workspace。

## Capabilities

### New Capabilities

- `macos-shell-workspace-persistence`: 定义 macOS shell workspace manifest、Space/Tab 持久化、Pinned Tab 快照、Unpinned Tab TTL、active-task 回收保护和损坏 manifest 恢复行为。

### Modified Capabilities

- `macos-shell-terminal-lifecycle`: 明确 terminal runtime 连续性只覆盖当前进程内仍属于 shell state 的 runtime；workspace manifest 恢复会启动新 terminal runtime，且 inactive unpinned Tab 的 lifecycle retirement 可以关闭对应 runtime。
- `macos-shell-build-test-contract`: 增加 manifest 创建/损坏恢复、Space 空状态、Pinned Tab 快照恢复、Unpinned Tab TTL 回收和 active-task 阻止回收的验证要求。

## Impact

- Apple client model/store: 新增 workspace manifest value types、manifest persistence store、manifest-to-shell-state materializer、TTL pruning 和 pin snapshot helpers。
- Apple client controller: `ShellHostController` 需要把 Space/Tab 创建、选择、关闭、pin/update pin、runtime activity metadata 同步到 manifest。
- Apple client UI: sidebar 需要显示空 Space，提供创建 Tab 入口，并提供 pin/unpin 或 update pin affordance。
- Terminal runtime metadata: 需要补充 terminal-aware active-task 信号，区分 foreground command / alan active session / idle shell。
- Existing shell state persistence: `ShellStateSnapshot` 继续服务当前 UI/control-plane/runtime 快照和诊断，但不再作为 workspace restore 权威。
- Tests/scripts: 需要增加 focused Swift tests 和 shell contract checks 覆盖 manifest lifecycle 与 terminal active-task 回收边界。
