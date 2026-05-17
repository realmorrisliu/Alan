## Context

当前 macOS shell 有两类状态被混在一个 `ShellStateSnapshot` 里：

- 用户工作区意图：Space / Tab / split 结构、选择状态、cwd。
- 当前运行态投影：terminal runtime metadata、renderer readiness、attention、alan binding、control-plane state file。

`ShellStatePersistenceStore` 已经能把 `ShellStateSnapshot` 写到 Application Support，但主窗口启动仍然使用 fresh bootstrap；即使切换成 restore，现有快照也不能表达 Pinned Tab、Unpinned Tab TTL、Space 永久存在、active-task 回收保护等产品语义。

本 change 将持久 workspace intent 从运行态快照里拆出来。`ShellWorkspaceManifest` 成为恢复 Space / Tab 的权威，`ShellStateSnapshot` 继续作为当前 UI、control plane、runtime projection 的可发布快照。

## Goals / Non-Goals

**Goals:**

- Space 创建后长期存在，直到用户显式删除。
- Tab 支持 pin/unpin；Pinned Tab 按 pin 时刻快照恢复。
- 未 pin Tab 在 12 小时内跨 App 重启恢复显示，超过 TTL 且 inactive 时自动回收。
- 回收判断使用 terminal-aware active-task 信号，而不是简单使用 child process 是否仍未退出。
- 允许空 Space，并在 UI 中保留可见的 Space 入口。
- 使用 versioned Codable JSON manifest，不引入 SwiftData、Core Data、SQLite 或 daemon-owned store。
- manifest 损坏时可恢复启动：旁路坏文件并创建 fresh default workspace。

**Non-Goals:**

- 不恢复退出前的 terminal 进程、scrollback 或 OS process；重启恢复只创建新的 terminal runtime。
- 不迁移旧 `shell-state-window_main.json` 到 workspace manifest。
- 不实现跨设备同步、多窗口 workspace 合并或 daemon/CLI 直接编辑 manifest。
- 不在本 change 中完成通用 content-container v0.2；但本设计需要避免阻塞后续 content-container state model。

## Decisions

1. **新增 `ShellWorkspaceManifest` 作为持久化权威。**

   Manifest 存放在 `Application Support/alan-macos/shell-workspace-window_main.json`，包含 `schemaVersion`、`windowID`、`selectedSpaceID`、`selectedTabID` 和 ordered spaces。它只表达用户希望长期保留的 workspace 结构，不保存 terminal renderer/runtime 对象。

   备选方案：继续持久化 `ShellStateSnapshot`。这改动最小，但会继续让 runtime projection、control-plane snapshot 和用户意图区分不清，后续 Pinned Tab 与 TTL 会在同一个模型里互相污染。

2. **Manifest 缺失或损坏不从旧 shell-state 迁移。**

   Manifest 不存在时创建默认 workspace：一个默认 Space 和一个默认 unpinned terminal Tab。Decode 失败时，将坏文件移动或复制为 `.corrupt-<timestamp>.json` 后创建 fresh default manifest。旧 `shell-state-window_main.json` 可以继续作为 control-plane/runtime 诊断输出，但不作为 workspace restore 输入。

   备选方案：从旧 `shell-state` 迁移。这个方案看似保留更多状态，但当前产品仍是 early development，且旧状态缺少 pin/TTL/lifecycle 字段；强迁移容易把 runtime 快照误解释为长期用户意图。

3. **Pin 是显式 restore snapshot，不是自动持续同步。**

   `pinSnapshot` 在用户 pin 或 update pin 时写入。pin 时是单 pane，就保存单 cwd 和 launch target；pin 时是 split，就保存当时 split tree 和每个 leaf pane 的 cwd/launch target。pin 之后的 cwd 变化、临时 split、pane move 不自动更新 pin snapshot。用户想持久化新的 split layout 时，需要重新 pin 或执行 update pin。

   备选方案：Pinned Tab 的布局永远跟随当前 tab。它适合“长期工作台”，但会把临时 split 也永久化，和用户确认的“要持久化 split 就重新 pin”语义不一致。

4. **Unpinned Tab 恢复用 live snapshot，生命周期由 TTL 管理。**

   未 pin Tab 持有 `liveSnapshot`，用于记录最近 cwd、launch target、可恢复 split layout 和最近标题。重启时，只要未超过 TTL 且 inactive，就 materialize 成新的 terminal pane/runtime。回收 anchor 是 `max(lastActivatedAt, lastActivityAt)`；超过 12 小时且没有 active task 才回收。

   备选方案：重启只恢复 pinned tabs。实现更简单，但不符合已确认的 Arc-like 行为，也会让用户短期工作上下文在重开 App 后消失。

5. **Active task 是 terminal-aware lifecycle 信号。**

   `processExited == false` 不能阻止回收，因为 idle login shell 也会一直未退出。需要引入或投影一个更精确的 active-task state：

   - foreground command running：active。
   - alan session running / pending / waiting for input：active。
   - idle prompt shell：inactive。
   - exited terminal：inactive。

   这个 state 可以先作为 pane/tab projection 写入 manifest，再由 prune 使用。实现可从 Ghostty shell integration metadata、alan binding、surface state 和 command lifecycle signal 组合得出。

   备选方案：所有未退出 process 都算 active。它会让 Unpinned Tab TTL 基本失效。

6. **Materializer 负责从 manifest 生成当前 shell state。**

   启动顺序：

   ```text
   load manifest
   -> create fresh default if missing/corrupt
   -> prune expired inactive unpinned tabs
   -> materialize ShellStateSnapshot
   -> publish control-plane state
   ```

   如果 selected tab 被 prune，materializer 选择当前 Space 的第一个 remaining tab；如果 Space 为空，则保留 selected Space，`selectedTabID = nil`，workspace UI 显示空状态。

7. **与 content-container change 解耦。**

   `generalize-macos-shell-content-containers` 会把当前 shell display/runtime state 从 pane-keyed terminal model 迁到 content-container model。这个 change 不等待它，也不把 workspace manifest 塞进那个大迁移里。Manifest 中的 restore snapshot 应通过小型 adapter materialize 到当前 shell state；如果 content-container 先落地，adapter 输出 v0.2 content state；如果本 change 先落地，adapter 输出当前 v0.1 terminal pane state。

## Risks / Trade-offs

- **Risk: manifest 与 runtime projection drift。** -> 只把用户意图和 lifecycle metadata 写入 manifest；runtime-only renderer/readiness/debug state 不进入 manifest。
- **Risk: active-task 信号不准确导致误清理。** -> 在 active-task 未能判定时默认保守保留，并用 tests 明确 idle shell 不应被误判为 active。
- **Risk: 空 Space UI 显得像 bug。** -> Sidebar 保留 Space 行；workspace 内容区提供 restrained empty state 和 new tab action。
- **Risk: 两套持久文件令人困惑。** -> 命名区分：`shell-workspace-*` 是 restore 权威；`shell-state-*` 是当前 control-plane/runtime snapshot。
- **Risk: content-container 后续迁移重复做 adapter。** -> Manifest restore snapshot 使用 content-neutral shape，避免把 terminal-only `ShellPane` 当作长期 schema。

## Migration Plan

1. 引入 manifest value types、store、corrupt-file handling 和 default manifest creation。
2. 引入 manifest materializer，把 manifest 输出为当前 `ShellStateSnapshot`。
3. 将 macOS primary shell startup 改为从 manifest restore，而不是 fresh `ShellStateSnapshot` bootstrap。
4. 将 Space/Tab 创建、选择、关闭、pin/unpin/update pin、runtime activity projection 同步到 manifest。
5. 添加 TTL prune 和 active-task protection。
6. 更新 UI 以支持空 Space 和 pin affordance。
7. 保留旧 `shell-state-window_main.json` 写入路径用于 control-plane/runtime diagnostics，直到后续 change 明确删除或替代它。

Rollback 是源代码级：移除 manifest startup path 后，App 可回到 fresh bootstrap 或旧 shell-state restore 行为。已创建的 manifest 文件保留在 Application Support 中，不需要 destructive migration。

## Open Questions

无。已确认：Unpinned Tab 采用 12 小时 Arc-like 恢复语义；Pinned Tab 是显式快照；Space 可为空；manifest 损坏时旁路并创建 fresh workspace；不迁移旧 shell-state。
