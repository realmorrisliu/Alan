# App Server Protocol Contract

> Status: VNext target contract（当前 HTTP/WS API 作为过渡层）。

## 目标

为 Alan 提供一个面向多客户端（TUI/Native/Web/IDE）的统一协议层，满足：

1. 长连接流式交互。
2. 明确的线程/轮次生命周期。
3. 稳定的事件订阅与恢复语义。
4. 输入分流（`steer/follow_up/next_turn`）与自治执行（scheduler/durable run）可扩展。

## 设计原则

1. **协议稳定优先**：客户端不依赖运行时内部结构。
2. **状态显式化**：thread/turn/item 一等对象。
3. **可恢复流**：断线后可基于事件游标补齐。
4. **向后兼容**：保留当前 `/sessions/*` API，逐步演进。

## 核心对象

### Thread

- 长寿命会话容器（可对应当前 session）。
- 包含 metadata、status、history index。
- 可携带 `task_id/run_id` 元数据（兼容追加字段）。

### Turn

- Thread 内一次执行轮次。
- 具有明确状态：running/yielded/completed/interrupted/failed。

### Item

- Turn 内原子条目：
  - user_input
  - queued_input（follow_up / next_turn）
  - assistant_delta/final
  - tool_call/tool_result
  - reasoning_delta
  - yield_request/resume
  - compaction marker

## 协议分层

### Control Plane（控制）

1. `thread/start|resume|fork|archive|rollback|compact`
2. `turn/start|input|interrupt|resume`
3. `tool governance` 相关应答（批准/拒绝/结构化输入）
4. `scheduler` 相关控制（可选扩展）：`run/sleep|run/wake|run/schedule`

### Data Plane（流）

1. `events/stream`（实时）
2. `events/read`（补偿拉取）
3. `thread/read`（快照读取）

## 当前 API 映射（兼容层）

当前 endpoints 可映射为：

1. `POST /sessions` -> `thread/start`
2. `POST /sessions/{id}/submit` -> `turn/start` / `turn/input`
3. `GET /sessions/{id}/events` -> `events/stream`
4. `GET /sessions/{id}/events/read` -> `events/read`
5. `POST /sessions/{id}/resume` -> `turn/resume`
6. `POST /sessions/{id}/rollback` -> `thread/rollback`
7. `POST /sessions/{id}/compact` -> `thread/compact`

兼容说明：

1. 历史 `turn/steer` 可作为 `turn/input{mode=steer}` 别名。
2. 历史无模式 `Op::Input` 默认映射 `mode=steer`。

## 输入模式（一等协议语义）

`turn/input` 建议结构：

1. `thread_id`
2. `input`（content parts）
3. `mode`：`steer | follow_up | next_turn`
4. `expected_turn_id`（可选并发保护）

语义：

1. `steer`：active turn 注入式引导。
2. `follow_up`：当前执行结束后处理。
3. `next_turn`：仅用于后续新 turn。

## 事件模型（规范建议）

每个事件必须包含：

1. `event_id`（单调递增或可排序）
2. `thread_id`
3. `turn_id`（若适用）
4. `type`
5. `timestamp`
6. `payload`

客户端恢复逻辑：

1. 记录 `latest_event_id`。
2. 重连后用 `after_event_id` 拉取缺口。
3. 若 `gap=true`，必须回退到 thread 快照重建状态。

## 生命周期约束

1. `turn/start` 后必须出现 turn 边界事件（started + terminal）。
2. `turn/input{mode=steer}` 只能作用于 active turn。
3. `turn/input{mode=follow_up|next_turn}` 允许入队到非 active 阶段（由队列语义处理）。
4. `turn/resume` 仅在 yielded 状态有效。
5. `turn/interrupt` 必须导致终态（interrupted 或 failed）。

## 错误语义

错误分两层：

1. **请求级错误**：参数非法、状态冲突、资源不存在。
2. **执行级错误**：运行时内部错误、provider 错误、tool 错误。

要求：

1. 请求级错误同步返回，包含可机读错误码。
2. 执行级错误进入事件流，带 `turn_id` 与错误上下文。
3. 队列超限（某输入模式）应作为可恢复请求级错误返回。

## 订阅与背压

1. 服务端应支持有界队列与过载保护。
2. 过载拒绝需返回可重试信号（明确错误码）。
3. 客户端需实现指数退避重试与断线恢复。

## 安全与治理

1. 批准请求与用户输入请求应走统一 Yield/Resume 通道。
2. 敏感操作必须可追溯到策略决策记录。
3. 协议层不应绕过 sandbox/policy 约束。
4. 恢复路径（recovery/replay）上的高风险动作不得跳过治理边界。

## 版本演进策略

1. 新字段优先“向后兼容追加”。
2. 破坏性修改需版本化（`v2`/`v3`）并提供迁移窗口。
3. schema/类型生成流程应纳入 CI 验证。
4. 输入语义扩展优先通过 `mode` 字段追加，避免频繁新增 method。

## 验收要点

1. 多客户端在同一 thread 上状态一致。
2. 断线恢复后无重复执行、无事件丢失（或可检测 gap）。
3. `steer/follow_up/next_turn` 行为在协议测试中可复现。
4. turn/input/resume/interrupt 行为在协议测试中可复现。
