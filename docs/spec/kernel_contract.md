# Alan Kernel Contract

> Status: V1 contract for long-term stability.  
> Scope: `alan-runtime` core behavior and invariants.

## 目标

这个文档定义 Alan 内核的不可变契约（invariants），用于约束后续开发：

- **内核保持小而稳**：只负责状态推进与执行控制，不承载业务策略。
- **行为可审计**：所有关键决策与副作用都可追溯。
- **扩展可替换**：通过 skills / tools / outer host 扩展能力，而不污染内核。

本文档优先级高于理念文档；协议细节以 `alan-protocol` 与 `alan-runtime` 源码为准。

## 边界定义

### 内核必须负责（MUST）

1. Tape 与 Rollout 的生命周期管理。
2. Turn 执行循环（LLM 生成、tool orchestration、yield/resume）。
3. Policy 决策接入与 sandbox 执行边界衔接。
4. 事件输出与输入操作（Op）的状态一致性。

### 内核不负责（MUST NOT）

1. 业务领域逻辑（如 coding agent 专有流程）。
2. 产品层 UI 协议细节（由 daemon/client 承担）。
3. 平台绑定集成语义（渠道、CRM、工单等）。

## 核心实体契约

### AgentConfig

- 定义“如何思考”：LLM、参数、工具集、治理配置。
- **无身份、无会话历史、无业务状态**。

### Workspace

- 定义“我是谁”：身份、persona、memory、skills、会话归档。
- 是可持久化上下文容器，不等价于单次执行。

### Session

- 定义“当前在做什么”：有界执行窗口。
- 持有 Tape、运行时状态、当前 turn 上下文。

### Tape

- 作为执行真值源（source of truth），记录消息与上下文片段。
- 允许显式压缩/回滚；禁止隐式丢失。

### Rollout

- 作为事件审计链，必须保留关键状态转换与工具决策轨迹。
- 应支持 replay/fork 所需的最小充分信息。

## 不变量（Invariants）

### 1) 状态推进单调性

- 同一 Session 内，turn 生命周期必须可判定：`started -> (yield/resume)* -> completed|error|interrupted`。
- 不允许出现“完成后再次恢复同一 turn”的非法状态。

### 2) 会话排他性

- 单个 Workspace 任一时刻仅允许一个 active runtime。
- 冲突必须在 hosting 层显式拒绝（例如返回冲突错误），不能静默覆盖。

### 3) 副作用显式化

- 所有外部副作用都必须通过 Tool 调用路径发生。
- 禁止在 LLM 生成路径中直接产生不可审计副作用。

### 4) 决策可追踪

- 每次工具决策必须可关联：策略来源、匹配规则、动作（allow/deny/escalate）、原因。
- `escalate` 必须进入可恢复的 `Yield -> Resume` 对称流程。

### 5) 上下文投影隔离

- Tape 是内部执行真值；provider 输入是投影视图。
- provider 适配差异不得反向污染 Tape 抽象。

### 6) 有界性优先

- 上下文窗口是硬约束；内核必须支持压缩、分段执行与会话切分。
- 不允许以“无限历史注入”规避窗口约束。

## 错误与恢复契约

1. **可恢复错误**：尽可能保留会话并继续后续 turn。
2. **不可恢复错误**：必须输出可诊断错误事件并停止当前执行。
3. **恢复入口**：通过显式 Op（如 `resume`）恢复，禁止隐式重入。

## 与扩展层的接口约束

1. Skills 仅通过提示注入与工具编排影响行为，不得绕过内核状态机。
2. 外部 Tool 实现可替换，但必须遵守统一 schema、timeout、capability 语义。
3. Host（CLI/daemon/app server）可扩展协议，但不得破坏内核 turn 语义。

## 兼容性策略

- 新能力默认通过“扩展点”落地，避免修改内核主循环。
- 若必须修改内核不变量，需同步更新：
  1. 本文档。
  2. `docs/testing_strategy.md` 对应契约测试说明。
  3. 迁移说明（breaking change）。

## 最小验收清单

一次内核相关改动，至少满足：

1. 新行为可映射到既有 turn 状态机，不新增隐式状态。
2. 工具副作用路径仍然唯一且可审计。
3. 协议事件序列在契约测试中可验证。
4. 回滚/压缩后不会破坏后续会话恢复。
