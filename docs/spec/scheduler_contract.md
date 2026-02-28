# Scheduler Contract (定时 / 休眠 / 唤醒 / 重启恢复)

> Status: VNext contract（定义 long-running 执行的调度真值语义）。

## 目标

Scheduler 是 Host/Daemon 层的系统能力，负责：

1. 定时触发 run 执行（reminder / cron-like / delay）。
2. 让 run 进入可恢复 `sleeping` 状态并在到点唤醒。
3. 在 daemon 或系统重启后恢复未终态调度项。

本合同只定义机制语义，不定义业务流程内容（由 skills 决定）。

## 作用域与边界

### Scheduler MUST

1. 持久化 schedule 与 run 唤醒状态。
2. 提供 at-least-once 的到点分发。
3. 通过幂等键保证“重复分发不重复副作用”。
4. 记录可审计调度事件链。

### Scheduler MUST NOT

1. 直接定义业务目标与步骤。
2. 绕过 runtime 状态机直接执行工具。
3. 覆盖 policy/sandbox 决策。

## 核心对象

### ScheduleItem

- `schedule_id`
- `task_id`
- `run_id`
- `trigger_type`（`at` / `interval` / `retry_backoff`）
- `next_wake_at`
- `status`（`waiting` / `due` / `dispatching` / `cancelled` / `completed` / `failed`）
- `attempt`
- `idempotency_key`

### SchedulerState（持久化最小字段）

- `last_dispatched_at`
- `last_completed_at`
- `last_error`
- `updated_at`

## 状态机（ScheduleItem）

1. `waiting -> due`：时间到达或条件满足。
2. `due -> dispatching`：调度器开始投递执行。
3. `dispatching -> waiting`：需再次触发（interval/backoff）。
4. `dispatching -> completed`：一次性任务完成。
5. `dispatching -> failed`：不可恢复失败。
6. `* -> cancelled`：显式取消。

约束：

1. `dispatching` 期间进程崩溃，重启后允许重复分发，但必须复用同一 `idempotency_key`。
2. `completed/cancelled` 为终态，不可自动回退到 `waiting`。

## 调度动作契约

### `schedule_at(run_id, wake_at, payload)`

1. 创建一次性 ScheduleItem。
2. `wake_at <= now` 时可立即标记 `due`。

### `sleep_until(run_id, wake_at)`

1. 将 run 状态切换为 `sleeping`。
2. 关联或创建对应 ScheduleItem。
3. 到点后恢复为 `running` 或进入执行队列。

### `retry_with_backoff(run_id, policy)`

1. 根据 `attempt` 计算下一次 `next_wake_at`。
2. 必须记录 backoff 计算输入（attempt、base、factor、max）。

### `on_boot_resume()`

1. daemon 启动后扫描所有 `waiting/due/dispatching` 调度项。
2. 将过期项标记 `due` 并重新入队。
3. 不得遗漏 `dispatching` 中断项。

## 与 Run 状态语义对齐

Scheduler 与 Run 的协同关系：

1. run `sleeping` 必须存在可追踪唤醒条件（时间或事件）。
2. run `yielded` 不由 scheduler 自动推进（需外部 resume）。
3. run `running` 不应被 scheduler 重复激活同一执行片段。

## 幂等与副作用边界

1. 每次调度分发必须附带稳定 `idempotency_key`（同一调度尝试一致）。
2. runtime/tool 层使用 `idempotency_key` 去重副作用调用。
3. 发生重复分发时，允许“重复计算”，禁止“重复不可逆副作用”。

## 恢复策略

重启恢复步骤：

1. 加载 ScheduleItem 持久化快照。
2. 对 `dispatching` 超时项执行归一化（回到 `due`）。
3. 对 `next_wake_at <= now` 的项批量推进到 `due`。
4. 恢复投递队列并发执行（受并发上限控制）。

## 观察性与审计

每个调度周期至少记录：

1. `schedule_id/run_id/task_id`
2. `trigger_type`
3. `wake_at/dispatched_at/completed_at`
4. `attempt/idempotency_key`
5. `result`（success/retry/cancel/fail）
6. `error`（若失败）

## 失败退化

1. 调度存储暂时不可写：拒绝新调度并返回可恢复错误。
2. 调度线程故障：应可自动重启，不影响已持久化数据。
3. 时钟漂移：记录 `clock_skew_detected` 警告，避免静默跳过任务。

## 验收要点

1. 到点任务在重启前后都可触发，不丢单。
2. 重复分发不会造成重复不可逆副作用。
3. `sleep_until` 与 run 状态切换一致、可审计。
4. `on_boot_resume` 可恢复 `dispatching` 中断任务。
