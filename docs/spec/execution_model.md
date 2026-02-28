# Execution Model (Task / Run / Session / Turn)

> Status: VNext target contract（兼容当前 Session/Turn 模型并引入自治执行语义）。

## 目标

Alan 需要同时支持：

1. 短交互（即时问答）。
2. 长周期自治执行（跨上下文窗口、跨时间片）。

为此，执行模型分层定义为：`Task -> Run -> Session -> Turn`。

## 对象分层

### Task（业务级目标）

- 表示一个完整目标委托（Goal + 约束 + Owner）。
- 生命周期通常跨多个 Run/Session。
- 典型字段：
  - `task_id`
  - `goal`
  - `constraints`（预算、策略、时间）
  - `owner`
  - `success_criteria`

### Run（一次执行尝试）

- Task 的一次可重试执行实例。
- 可因中断、休眠、超时、策略升级而结束，再创建下一次 Run。
- 典型字段：
  - `run_id`
  - `task_id`
  - `attempt`
  - `started_at` / `ended_at`
  - `status`（`pending/running/sleeping/yielded/succeeded/failed/cancelled`）

### Session（有界上下文容器）

- Run 在某个时间片内的执行窗口。
- 受模型 context window 限制，可被压缩/归档/切换。
- 典型字段：
  - `session_id`
  - `run_id`
  - `workspace_id`
  - `tape`
  - `rollout`

### Turn（最小状态推进单元）

- 一次 `Op::Turn` 触发的执行过程。
- 包含：输入、LLM 生成、tool batch、yield/resume、结束事件。

## 当前实现与目标映射

当前 Alan 以 Session/Turn 为主。映射关系：

- 当前 `Session` ~= 目标模型中的 `Session`
- 当前“一个会话内持续交互”可视为同一 `Run`
- `Task` 目前尚未成为协议一等对象，由上层编排承担

迁移原则：在不破坏现有 Op/Event 语义前提下，引入 Task/Run 元数据。

补充合同：

1. 调度语义见 [`scheduler_contract.md`](./scheduler_contract.md)
2. 输入分流语义见 [`interaction_inbox_contract.md`](./interaction_inbox_contract.md)
3. 恢复与幂等语义见 [`durable_run_contract.md`](./durable_run_contract.md)

## Turn 状态机

### 状态

1. `Idle`：等待输入。
2. `Running`：执行 LLM + tools。
3. `Yielded`：等待外部 `Resume`。
4. `Completed`：本 turn 正常结束。
5. `Interrupted`：被显式中断。
6. `Failed`：不可恢复错误。

### 转移

1. `Idle --(Op::Turn)--> Running`
2. `Running --(policy escalate / virtual input request)--> Yielded`
3. `Yielded --(Op::Resume)--> Running`
4. `Running --(done)--> Completed`
5. `Running --(Op::Interrupt)--> Interrupted`
6. `Running|Yielded --(fatal error)--> Failed`

## 操作语义契约

### `turn`

- 开始新 turn，建立本轮边界。
- 必须发出 `turn_started` 与最终边界事件（`turn_completed` 或错误边界）。

### `input`（一等输入模式）

`input` 必须支持三种模式：

1. `steer`：对当前运行 turn 的插入式引导，不是新 turn。
2. `follow_up`：当前执行结束后再处理。
3. `next_turn`：只影响后续新 turn，不触发当前 turn 立即重排。

约束：

1. 工具批处理中至少每次 tool 完成后检查 `steer`。
2. `follow_up/next_turn` 不得破坏当前 turn 的因果顺序。
3. `yielded` 状态仍以 `resume` 为唯一推进入口。

### `resume`

- 仅对处于 `Yielded` 的 turn 有效。
- 必须带 `request_id`，与挂起请求一一对应。

### `interrupt`

- 尽快终止当前执行。
- 终止后 turn 进入 `Interrupted`，并可继续后续 turn。

### `compact`

- 对会话上下文执行压缩，释放窗口压力。
- 必须是显式操作或可解释的自动策略触发。

### `rollback`

- 回滚最近 N 个 turn 的可回滚状态。
- 必须写入可审计标记，避免“静默历史改写”。

## 并发与排队

1. 同一 Session 同时只能有一个 active turn。
2. 同一 Workspace 只允许一个 active runtime（host 层约束）。
3. 输入排队优先级建议：`steer > follow_up > next_turn`。
4. 队列超限必须可见报错，不得静默丢弃。

## 恢复与重放

1. Session/Run 恢复后，必须能判定最近 turn 的终态。
2. `Yielded` 状态可在恢复后继续等待 `resume`。
3. `Sleeping` run 必须由调度器唤醒（不可隐式继续）。
4. replay 语义要区分：
  - 仅重放事件（不重复副作用）
  - 重执行（需要幂等保护）

## 迁移计划（建议）

1. **阶段 1**：在 Session 元数据中引入 `run_id/task_id`（可选）。
2. **阶段 2**：为 `input` 增加 `mode`（默认 `steer`，兼容旧行为）。
3. **阶段 3**：引入 scheduler + durable run checkpoint。
4. **阶段 4**：将自治场景纳入 harness 发布门禁。

## 验收要点

1. turn 状态机无歧义、可测试。
2. `steer/follow_up/next_turn` 语义稳定且互不冲突。
3. steering/resume/interrupt 三条控制路径相互不冲突。
4. 跨 session/run 恢复不会导致重复执行副作用。
