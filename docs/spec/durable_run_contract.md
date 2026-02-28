# Durable Run Contract (Checkpoint / Idempotency / Side-Effect Recovery)

> Status: VNext contract（定义 run 级连续性与副作用安全恢复语义）。

## 目标

Durable Run 解决的问题是：

1. 进程或系统重启后，run 能继续推进而不是丢失上下文。
2. 恢复过程中不重复执行不可逆副作用。
3. replay / rollback / fork 语义彼此不冲突。

## 范围与边界

### Durable Run MUST

1. 持久化 run 关键执行状态（checkpoint）。
2. 给副作用动作绑定幂等键与结果记录。
3. 提供恢复流程（bootstrap -> reconcile -> resume）。

### Durable Run MUST NOT

1. 依赖模型“记住”恢复语义。
2. 通过非审计路径补写历史。
3. 在恢复时自动跳过 governance 边界。

## 核心对象

### RunCheckpoint

- `checkpoint_id`
- `task_id`
- `run_id`
- `session_id`
- `turn_id`（若存在）
- `run_state`（`running/sleeping/yielded/...`）
- `pending_yield`（可选）
- `next_action_hint`
- `created_at`

### EffectRecord

- `effect_id`
- `run_id`
- `tool_call_id`
- `idempotency_key`
- `effect_type`（file/network/process/...）
- `request_fingerprint`
- `result_digest`
- `status`（`applied` / `failed` / `unknown`）
- `applied_at`

## Checkpoint 写入时机

至少在以下边界写 checkpoint：

1. turn 开始后（拿到可恢复执行入口）。
2. 每次进入 `yielded/sleeping` 前。
3. 每次关键副作用确认后（effect record 同步落盘）。
4. turn 终态后（completed/failed/interrupted）。

要求：

1. checkpoint 与 effect record 顺序可重建因果关系。
2. 落盘失败必须可见（warning/error），不可静默忽略。

## 恢复流程契约

`restore_run(run_id)` 建议流程：

1. 读取最新 RunCheckpoint。
2. 校验 run 状态合法性并归一化中间态。
3. 重建 pending yield / scheduler 关联（若有）。
4. 将 runtime 定位到可恢复入口并继续执行。

归一化示例：

1. 崩溃时处于“副作用调用中”：标记 `effect status=unknown` 并触发去重查询。
2. 崩溃时处于 `dispatching`：回退为可重试状态，不丢弃 run。

## 幂等语义

1. 同一逻辑副作用必须复用同一 `idempotency_key`。
2. 恢复时若检测到 `idempotency_key` 已成功应用：
  - 允许跳过真实执行；
  - 必须写入“命中去重”的审计记录。
3. 对无法验证状态的副作用（`unknown`）：
  - 默认进入人工/策略保护路径（escalate）或安全重试策略。

## 副作用恢复策略

按副作用类型建议策略：

1. **文件写入（可比对）**：通过内容哈希/mtime/fingerprint 判重。
2. **网络调用（外部系统）**：优先依赖外部幂等 API + 本地 effect record。
3. **进程执行（shell）**：对不可幂等命令默认保守策略（需边界确认）。

## 与 Replay / Rollback / Fork 的关系

### Replay

1. 默认 replay 仅重放事件，不重复副作用。
2. 若显式 re-execute，必须重新生成 run，并进入幂等保护路径。

### Rollback

1. rollback 仅修改可回滚上下文，不得删除 effect 审计链。
2. rollback 后再次执行同类动作仍受 idempotency 保护。

### Fork

1. fork 继承必要上下文与摘要，但不继承“已应用副作用”的执行权。
2. fork run 的副作用幂等键命名空间应隔离（避免误判重复）。

## 与治理边界协同

1. 恢复路径上的高风险动作同样走 policy + boundary。
2. “恢复中”不等于“免审批”。
3. 所有自动恢复决策需要可审计 reason。

## 观察性与审计

至少产出以下可检索字段：

1. `run_id/checkpoint_id/effect_id`
2. `recovery_attempt`
3. `dedupe_hit`（bool）
4. `decision`（resume/retry/escalate/fail）
5. `reason`

## 失败退化

1. checkpoint 不可读：run 标记 `failed_recovery`，要求人工介入。
2. effect 状态不可判定：进入安全模式，不直接继续高风险步骤。
3. 连续恢复失败超过阈值：停止自动重试并报警。

## 验收要点

1. 重启后 run 能从 checkpoint 继续。
2. 已成功副作用不会因恢复重复执行。
3. rollback/fork/replay 不破坏副作用审计链。
4. 恢复决策可完整追踪。
