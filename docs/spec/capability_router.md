# Capability Router Contract

> Status: VNext contract（定义 runtime 如何在 builtin / extension / bridge 之间统一路由能力调用）。

## 目标

Capability Router 负责把“我要调用什么能力”与“能力由谁实现”解耦：

1. runtime 只依赖 capability 名称，不依赖具体实现位置。
2. 支持本地 provider、远端 bridge provider 的统一调用语义。
3. 把治理、幂等、超时、审计放在同一调用管道中。

## 非目标

1. 不定义具体业务流程（由 skills 决定）。
2. 不取代 policy/sandbox（router 只接入治理，不改治理真值）。
3. 不要求一次性支持所有 provider 发现机制。

## 角色与职责

### Router MUST

1. 根据 capability 选择 provider 并执行路由。
2. 统一注入 `idempotency_key`、`deadline`、`trace_context`。
3. 统一产出调用事件与审计字段。
4. 在可安全场景下执行有限 fallback。

### Router MUST NOT

1. 绕过 `PolicyEngine` 直接执行 side-effect capability。
2. 在 side-effect 已发生后做“静默重试+换 provider”。
3. 修改 turn 状态机语义。

## 核心对象

### CapabilityCall

1. `call_id`
2. `task_id/run_id/session_id/turn_id`
3. `name`（capability 名称）
4. `input`
5. `side_effect_mode`：`none | reversible | irreversible`
6. `idempotency_key`（按 capability 要求）
7. `deadline_ms`
8. `route_mode`：`strict | best_effort | shadow`

### ProviderRef

1. `provider_id`
2. `source`：`builtin | extension_local | extension_bridge`
3. `priority`
4. `health_status`
5. `supports`（capability 列表 + 版本）
6. `cost_class`（可选）

### RouteDecision

1. `selected_provider`
2. `fallback_chain`
3. `policy_action`：`allow | deny | escalate`
4. `reason`

## 注册与发现

Router 维护统一 registry，来源包括：

1. runtime 内建 provider（当前 builtin tools）。
2. Extension Host 注册的本地 provider。
3. Harness Bridge 注册的远端 provider。

约束：

1. provider 必须先通过 manifest/compatibility 校验才能注册。
2. 同名 capability 可多 provider 并存，但必须有稳定优先级规则。

## 路由算法（建议）

1. 归一化输入 capability 名称与版本约束。
2. 从 registry 查找候选 provider 列表。
3. 以 `PolicyEngine` 评估调用请求（含 risk/context）。
4. `deny/escalate` 直接返回治理结果，不执行 provider。
5. 在 `allow` 前提下按评分选择 provider：
  - 首选健康、低延迟、同机 provider；
  - 再按 `priority` 与 `cost_class` 排序。
6. 派发调用并等待结果（受 `deadline_ms` 约束）。
7. 根据返回状态决定是否 fallback（仅限安全条件）。
8. 记录事件与 effect 审计。

## Fallback 规则

1. `side_effect_mode=none` 可按 `best_effort` fallback。
2. `reversible` 默认不自动 fallback，除非 capability 声明支持事务回滚。
3. `irreversible` 禁止自动 fallback；仅可走人工/策略路径。
4. `shadow` 模式只用于评估，不得产生真实副作用。

## 幂等与副作用

1. Router 必须把 `idempotency_key` 透传到 provider。
2. 同一 `idempotency_key` 命中去重时，返回 `dedup_hit` 并写审计事件。
3. side-effect capability 成功后必须写入 `effect_refs` 并关联 `call_id`。

## 与 Turn / Run 语义的关系

1. Router 是 turn 内部机制，不应引入额外隐式 turn。
2. 调用超时/失败应映射到当前 turn 的可恢复错误或 yield 路径。
3. `run` 恢复后再次路由同一副作用调用时，必须复用原幂等键。

## 与 Input 模式语义的关系

1. `steer` 触发重规划时，尚未执行的 capability call 可取消。
2. `follow_up/next_turn` 仅影响后续规划，不直接改变正在执行的 call。
3. `yielded` 状态下 Router 不自动重入，等待明确 `resume`。

## 事件与可观测性

建议事件（或 rollout 等价字段）：

1. `capability_route_selected`
2. `capability_call_started`
3. `capability_call_completed`
4. `capability_call_failed`
5. `capability_call_deduped`
6. `capability_route_fallback`

每条事件建议包含：

1. `call_id/provider_id/capability`
2. `run_id/session_id/turn_id`
3. `policy_action`
4. `latency_ms/status`

## 性能与背压

1. Router 应支持并发上限与队列保护，避免 provider 风暴。
2. provider 过载时返回可重试错误，不可无限阻塞 turn。
3. 对长尾 provider 允许启用熔断与短期降级。

## 错误语义

1. `provider_unavailable`：可重试或 fallback。
2. `capability_not_found`：请求级错误，直接失败。
3. `policy_denied` / `policy_escalated`：治理结果，不进入 provider 执行。
4. `deadline_exceeded`：执行级错误，可映射 retry/backoff。

## 与 Harness 的关系

Harness 至少覆盖：

1. 多 provider 选择一致性（同输入同决策）。
2. side-effect 场景下无非法 fallback。
3. 去重命中与恢复后重试语义正确。
4. bridge/local 混部时事件与审计字段一致。

## 分阶段落地建议

1. Phase 1：把 builtin tools 接到 Router（单 provider）。
2. Phase 2：接入 extension local provider（多 provider）。
3. Phase 3：接入 bridge provider（远端调度 + 恢复）。

## 验收要点

1. runtime 调用 capability 不依赖具体 provider 位置。
2. 高风险调用在路由层不会绕过治理边界。
3. side-effect 调用在失败/恢复场景下仍保持幂等。
4. router 决策可回放、可审计、可测试。

