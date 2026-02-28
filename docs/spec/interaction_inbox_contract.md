# Interaction Inbox Contract (steer / follow_up / next_turn)

> Status: VNext contract（将人类输入从“单一 input”升级为三类一等语义）。

## 目标

在不破坏 turn 一致性的前提下，实现 human I/O 与 agent I/O 的并存：

1. 人类可在执行中注入高优先级引导（`steer`）。
2. 人类可提交完成后处理的增量需求（`follow_up`）。
3. 人类可提交仅作用于后续回合的上下文意图（`next_turn`）。

## 输入分类

### `steer`

- 用途：打断当前执行路径并重规划。
- 时机：active turn 期间可接收。
- 语义：尽快在可安全中断点注入。

### `follow_up`

- 用途：当前执行结束后立刻处理。
- 时机：可在 active turn 期间排队。
- 语义：不打断当前执行主路径。

### `next_turn`

- 用途：作为下一个用户回合的背景输入。
- 时机：任意时刻可入队。
- 语义：不触发立即执行，不打断当前 turn。

## 传输表示（协议建议）

`turn/input` 或 `Op::Input` 增加模式字段：

1. `mode = steer | follow_up | next_turn`
2. 默认值建议为 `steer`（兼容现有行为）

兼容映射：

1. 旧 `turn/steer` 视为 `turn/input{mode=steer}`。
2. 旧无模式 `Op::Input` 视为 `mode=steer`。

## 队列与优先级

建议维护三个逻辑队列：

1. `Q_steer`（最高优先）
2. `Q_follow_up`
3. `Q_next_turn`

优先级规则：

1. tool 批次边界先检查 `Q_steer`。
2. turn 终态后检查 `Q_steer`，再检查 `Q_follow_up`。
3. `Q_next_turn` 仅在创建新用户 turn 时注入。

## 执行行为矩阵

### Active turn + tool batch 中

1. 收到 `steer`：允许跳过剩余可跳过 tool，并注入 steer 后继续同一 turn。
2. 收到 `follow_up`：仅排队，不中断。
3. 收到 `next_turn`：仅排队，不中断。

### Active turn + yielded 中

1. `resume` 仍是唯一推进 yielded 的输入。
2. `steer/follow_up/next_turn` 可入队，但不替代 `resume`。

### Idle（无 active turn）

1. `steer/follow_up` 可配置为触发新 turn（若 `trigger_turn=true`）。
2. `next_turn` 默认仅缓存，等待后续显式 `turn/start`。

## 一致性约束

1. 同一 session 同时仅允许一个 active turn。
2. 输入排队不得重排已提交 `resume` 的因果顺序。
3. turn 内注入必须有审计痕迹（source/mode/enqueued_at/applied_at）。

## 背压与容量

1. 每类队列应有上限（例如 `steer <= 16`）。
2. 超限时拒绝最新输入并返回可恢复错误。
3. 拒绝事件应写入可观测 warning/error（不可静默丢弃）。

## 事件建议

建议增加可选事件（或在现有事件 payload 扩展字段）：

1. `input_queued`：`{mode, queue_size}`
2. `input_applied`：`{mode, turn_id}`
3. `input_dropped`：`{mode, reason}`

若暂不新增事件，至少在 rollout 中记录等价审计项。

## 与规划质量的关系

为了避免“feature1 完成后才看见 feature2”：

1. `follow_up` 入队后可参与“未来意图预览”上下文。
2. 预览只影响规划，不强制立即执行 follow_up。
3. 必须标注来源为 queued intent，避免与明确用户当前指令混淆。

## 迁移建议

1. 第一阶段先引入 `mode` 字段与默认兼容。
2. 第二阶段把 `turn/steer` 标记为兼容别名。
3. 第三阶段在 harness 中把三类输入语义做回归门禁。

## 验收要点

1. `steer` 在工具批次中可稳定中断并重规划。
2. `follow_up` 不阻塞当前执行，且当前执行完成后可被消费。
3. `next_turn` 不触发即时执行，且在下一轮可见。
4. 队列超限行为可预测、可审计。
