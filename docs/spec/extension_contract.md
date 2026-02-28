# Extension Contract (Plugin / Extension Host)

> Status: VNext contract（定义 Alan 的扩展机制与生命周期契约）。

## 目标

在保持 `alan-runtime` 内核稳定的前提下，引入可插拔能力层，使 Alan 可以：

1. 通过 extension 增加/替换能力，而不改内核主循环。
2. 让 skills 编排 extension 能力，而不是把系统能力编码进 prompt。
3. 在本地与远程（bridge）模式下保持统一调用与治理语义。

## 非目标

1. 不把 extension 变成新的业务流程引擎（流程编排仍由 skills/runtime 完成）。
2. 不允许 extension 绕过 policy/sandbox 直接执行高风险动作。
3. 不在 VNext 强制绑定单一插件打包格式或语言。

## 术语

1. **Extension**：可加载能力单元（工具、memory、channel、domain module）。
2. **Extension Host**：负责 extension 生命周期、隔离、健康检查的宿主。
3. **Capability**：可被路由调用的能力接口（例如 `tool.read_file`、`memory.search`）。
4. **Capability Router**：运行时路由层，决定调用哪个 provider（见 `capability_router.md`）。
5. **Bridge**：跨进程/跨机器的 extension 托管通道（见 `harness_bridge.md`）。

## 分层定位

1. `kernel/runtime`：状态机、幂等、治理边界、不变量。
2. `extension`：新增能力的实现与外部系统集成。
3. `skills`：能力编排与策略（workflow）。
4. `harness`：验证 extension 与系统行为是否稳定可回归。

## Extension 类型（VNext）

1. `tool_provider`
  - 提供可执行能力（读写文件、命令、外部 API）。
2. `memory_provider`
  - 提供长期记忆读写与检索后端。
3. `channel_adapter`
  - 提供外部交互通道（通知、移动端控制、Webhook）。
4. `domain_module`
  - 提供领域能力集合（coding/research/ops 等）。

说明：类型用于治理与观测分类，不限制内部实现语言。

## Manifest Contract（草案 v0）

每个 extension 必须提供 manifest，至少包含：

1. `id`：稳定唯一标识（建议反向域名）。
2. `version`：extension 版本（semver）。
3. `contract_version`：本契约版本（例如 `0.x`）。
4. `kind`：`tool_provider | memory_provider | channel_adapter | domain_module`。
5. `entrypoint`：启动入口（本地可执行或 bridge endpoint）。
6. `capabilities[]`：能力声明（名称、版本、风险、schema）。
7. `permissions`：最小权限声明（fs/network/process/scheduler）。
8. `config_schema`：配置 JSON Schema（可选）。
9. `state_namespace`：私有状态命名空间（必填）。
10. `healthcheck`：健康检查端点或调用名（必填）。

示例：

```yaml
id: io.alan.coding.base
version: 0.1.0
contract_version: 0.1
kind: domain_module
entrypoint:
  mode: local_process
  command: ["alan-ext-coding", "serve"]
state_namespace: ext/io.alan.coding.base
capabilities:
  - name: tool.code_edit
    version: 1
    effects: [write, process]
    risk_level: B
    idempotency: required
permissions:
  fs: workspace_only
  network: deny
  process: allowlist
healthcheck:
  probe: ext.health
```

## Capability 声明契约

每个 capability 必须声明：

1. `name` 与 `version`。
2. `effects`：`read | write | network | process | memory | channel | scheduler`。
3. `risk_level`：`A | B | C`（映射治理边界）。
4. `input_schema` / `output_schema`（机读约束）。
5. `timeout_ms`（默认超时）。
6. `idempotency`：`required | optional | unsupported`。

规则：

1. 声明为 `required` 的 capability 必须接受并正确处理 `idempotency_key`。
2. `risk_level C` 默认进入 `escalate` 路径，除非策略明确放行。

## 生命周期契约

Extension Host 必须支持以下生命周期：

1. `load`：读取 manifest 与配置，完成兼容性检查。
2. `init`：注入运行上下文（workspace、policy、trace）并初始化资源。
3. `start`：进入可服务状态并注册 capabilities。
4. `stop(reason)`：优雅停止，处理在途请求。
5. `recover(checkpoint_ref)`：重启后恢复本地状态（可选）。
6. `health`：健康检查与版本信息。

约束：

1. `start` 失败不得导致 runtime 崩溃；应进入隔离降级模式。
2. `stop` 必须可中断，避免阻塞 daemon 关闭。
3. 生命周期事件必须写入审计链。

## 调用契约

### Request

`CapabilityRequest` 至少包含：

1. `request_id`
2. `task_id/run_id/session_id/turn_id`
3. `capability`
4. `input`（JSON payload）
5. `idempotency_key`（按 capability 要求）
6. `deadline_ms`
7. `trace_context`
8. `governance_context`（策略决策摘要）

### Response

`CapabilityResponse` 至少包含：

1. `request_id`
2. `status`：`ok | dedup_hit | retryable_error | fatal_error | denied | escalated`
3. `output`
4. `effect_refs`（产生副作用时必填）
5. `error`（失败时必填）
6. `retry_after_ms`（可重试错误可选）

规则：

1. `request_id` 必须幂等可追踪。
2. 遇到取消/超时信号时，extension 必须尽快返回。
3. 已完成不可逆副作用时必须返回可审计 `effect_refs`。

## 治理与沙箱

1. Router 先执行 `policy -> allow/deny/escalate`，再进行 capability 调用。
2. extension 不得自行绕过治理路径。
3. extension 声明权限是“上限建议”，实际执行仍受 sandbox 物理边界约束。
4. 恢复路径与正常路径适用同一治理策略（不可“恢复免审批”）。

## 状态与持久化边界

1. extension 私有状态写入：
  - `{workspace}/.alan/extensions/{extension_id}/state/`
2. 临时文件写入：
  - `{workspace}/.alan/extensions/{extension_id}/tmp/`
3. extension 不得私自改写 rollout/checkpoint 真值文件。
4. 需要跨重启恢复的 extension 状态必须版本化并可迁移。

## 隔离等级（建议）

1. `tier_local_process`（默认）：本机子进程，低延迟，受宿主管理。
2. `tier_remote_bridge`：远端进程，通过 bridge 调用，适合移动端/云托管。
3. `tier_in_process`（仅开发）：仅用于实验，不作为生产默认。

## 观测与审计

每次 capability 调用至少记录：

1. `extension_id/capability/request_id`
2. `run_id/session_id/turn_id`
3. `route`（local/bridge）
4. `latency_ms/status`
5. `dedupe_hit`（bool）
6. `policy_action/risk_level`

## 错误与退化策略

1. `retryable_error`：允许有界重试（指数退避）。
2. `fatal_error`：标记 extension 不健康并触发隔离。
3. `denied/escalated`：属于治理结果，不计入 extension 可用性故障。
4. 连续故障超过阈值触发 circuit breaker，并回退到替代 provider（若可用）。

## 版本与兼容性

1. `contract_version` 主版本不兼容时，Host 必须拒绝加载。
2. capability schema 破坏性修改必须升 `major`。
3. 废弃 capability 应提供迁移窗口与兼容别名。

## 分阶段落地建议

1. Phase 1：现有 builtin tools 包装为 `tool_provider` 语义。
2. Phase 2：引入 `memory_provider` 与 `channel_adapter`。
3. Phase 3：通过 `harness_bridge` 托管远端 extension。

## 验收要点

1. 不修改 runtime 主循环即可新增/替换 capability provider。
2. extension 故障不会导致 session 状态机损坏。
3. 所有高风险 capability 调用仍受治理边界约束。
4. 本地/远程调用在审计字段与幂等语义上保持一致。

