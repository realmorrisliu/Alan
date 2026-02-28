# Harness Bridge Contract

> Status: VNext contract（定义 Alan 在本地/远程实例间的控制与能力桥接协议）。

## 目标

Harness Bridge 是 Alan 的“运行与治理平面”扩展，用于：

1. 远程控制任意 Alan 实例（本机、家庭电脑、云主机）。
2. 托管跨进程/跨机器 capability provider。
3. 在断线、重启、网络抖动下保持可恢复执行。

Bridge 不替代 runtime 状态机；它为 runtime 提供可持续运行与远程连接能力。

## 非目标

1. 不把 bridge 变成新的业务协议层（业务语义仍由 App Server / Op/Event 承担）。
2. 不绕过目标节点上的 policy/sandbox。
3. 不在 VNext 直接引入多租户云控制台的全部功能。

## 架构角色

1. `Bridge Controller`（daemon 内）
  - 管理连接、鉴权、路由、重连恢复。
2. `Bridge Node Agent`（目标机器）
  - 与本地 runtime/extension host 对接，执行请求。
3. `Relay`（可选）
  - 处理 NAT/移动网络场景下的中继连接。
4. `Client`（TUI/Native/Web/手机）
  - 通过 app server 发起控制与订阅请求。

## 数据面与控制面

### 控制面（Control Plane）

1. `bridge.register`
2. `bridge.authenticate`
3. `bridge.heartbeat`
4. `bridge.attach_session`
5. `bridge.detach_session`
6. `bridge.drain`

### 数据面（Data Plane）

1. `bridge.call`（capability 调用）
2. `bridge.result`（结果返回）
3. `bridge.event`（事件流转发）
4. `bridge.cancel`（取消在途调用）
5. `bridge.resume`（断连后重放游标恢复）

## 消息信封契约（草案）

每条 bridge 消息建议包含：

1. `bridge_id`
2. `node_id`
3. `message_id`
4. `seq`（单调序号）
5. `ack`（已确认到的对端序号）
6. `timestamp`
7. `type`
8. `payload`
9. `trace_context`

要求：

1. `seq` 必须单调，便于断线后补偿。
2. `ack` 必须显式，避免“以连接是否存在推断送达”。

## 连接与恢复语义

### 建链

1. Node 发起 `register + authenticate`。
2. Controller 下发会话与能力授权范围。
3. 双方进入 heartbeat 循环。

### 断线恢复

1. 任一方重连时提交 `last_acked_seq`。
2. 对端按游标重放未确认消息。
3. 在途 `bridge.call` 依据 `call_id + idempotency_key` 去重恢复。

### 节点重启

1. Node 重启后必须重新注册并同步健康状态。
2. Controller 对未终态任务执行 reconcile：
  - 可恢复的继续派发；
  - 不可判定的进入人工/策略路径。

## 一致性与交付语义

1. Bridge 调用交付语义为 at-least-once。
2. 不可逆副作用的“exactly-once”依赖幂等键与 EffectRecord（见 durable run contract）。
3. 同一 `call_id` 的重复消息不得导致重复不可逆执行。

## 与 App Server 协议对齐

1. 客户端仍通过 `thread/turn/input/resume/interrupt` 语义交互。
2. Bridge 只影响调用与事件的传输路径，不改变 Op/Event 语义。
3. `steer/follow_up/next_turn` 队列语义在目标节点保持一致。

## 与 Capability Router 对齐

1. Router 可把 provider 源标记为 `extension_bridge`。
2. 路由决策应综合节点健康、延迟、策略与能力版本。
3. Bridge 失败可触发安全 fallback（仅限无副作用调用）。

## 安全模型

1. 鉴权：
  - 建议短期令牌 + 节点长期身份（轮换支持）。
2. 授权：
  - capability 级 scope（最小权限）。
3. 策略：
  - 最终执行策略以目标节点 policy 为准。
4. 审计：
  - 记录 `who -> where -> what -> why -> result` 全链路。

禁止事项：

1. 未授权节点不得附着已存在会话。
2. Bridge 令牌不得直接授予“绕过 governance”能力。

## 观测与 SLO 指标

建议最小指标：

1. `bridge_connected_nodes`
2. `bridge_heartbeat_lag_ms`
3. `bridge_reconnect_count`
4. `bridge_call_latency_ms`
5. `bridge_call_timeout_rate`
6. `bridge_replay_gap_count`

建议日志字段：

1. `bridge_id/node_id/session_id/run_id/turn_id/call_id`
2. `seq/ack`
3. `route/policy_action/status`

## 故障与退化策略

1. Relay 不可用：
  - 本地节点继续工作，远程控制降级。
2. Bridge Controller 重启：
  - 节点自动重连并基于游标恢复。
3. 长时间离线：
  - 任务转为 `degraded` 并保留可恢复上下文。
4. 重放缺口不可修复：
  - 标记 `gap_detected`，要求回退快照重建。

## 分阶段落地建议

1. Phase 1（单机）
  - 同机进程桥接，验证连接/重放/幂等链路。
2. Phase 2（远程节点）
  - 引入 relay，支持手机控制桌面/云实例。
3. Phase 3（多节点）
  - 多实例路由、故障转移、统一观测看板。

## 与 Alan 哲学对齐

1. Turing 机语义仍在 runtime：Bridge 不改变状态机，只扩展“执行场所”。
2. UNIX 风格组合：Bridge 是可替换通道，不是业务逻辑内核。
3. Human-in-the-End：远程控制增强的是 owner 介入能力，不是审批泛滥。

## 验收要点

1. 手机/远程客户端可稳定控制目标 Alan 实例。
2. 断线重连后调用与事件可基于游标恢复，不丢关键信息。
3. 重复投递不会重复不可逆副作用。
4. 远程路径不绕过目标节点治理边界。

