# Governance Boundaries (Commit Boundaries)

> Status: VNext governance contract（建立 HITE 的“异常接管”执行面）。

## 目标

把“人类审批每一步”升级为“在关键提交边界接管”：

`Human Defines -> Agent Executes -> Human Owns`

核心是把高风险不可逆动作收敛成可声明、可执行、可审计的边界。

## 边界分级

### Level A: Routine（常规）

- 低风险、可逆、低影响动作。
- 默认策略：`allow`。

示例：读文件、本地静态分析、临时草稿生成。

### Level B: Sensitive（敏感）

- 有副作用但可控，可能影响质量或成本。
- 默认策略：`escalate` 或受限 `allow`。

示例：批量写文件、外部网络调用、非生产环境部署。

### Level C: Commit Boundary（关键提交边界）

- 高风险、不可逆、具法律/资金/生产影响。
- 默认策略：`escalate`（必要时 `deny`）。

示例：生产发布、真实支付、删除核心数据、推送主分支。

## 风险判定维度

策略引擎评估建议至少包含：

1. 能力类型（read/write/network/unknown）。
2. 作用对象（路径、环境、资源域）。
3. 影响半径（文件数、变更行数、目标系统）。
4. 可逆性（是否可 rollback）。
5. 成本/预算（时间、token、金钱）。

## Policy-as-Code 建议扩展

在当前 `allow/deny/escalate` 基础上，建议扩展字段：

1. `risk_level`：A/B/C
2. `boundary`：是否属于关键提交边界
3. `requires_owner`：是否必须 owner 级确认
4. `max_impact`：影响半径上限
5. `budget_guard`：成本阈值

说明：这些字段可由策略文件声明，也可由运行时动态补充计算。

## 交互契约

当命中 `escalate`：

1. 运行时必须发出 `Yield`，包含：
  - `request_id`
  - `action_summary`
  - `risk_reason`
  - `suggested_options`
2. 外部通过 `Resume` 返回明确决策：allow/deny + 可选修改条件。

禁止“自动降级绕过”：一旦进入边界流程，不可静默回到 allow。

## 审计链要求

每个关键边界决策应记录：

1. `policy_source`（builtin/workspace/custom）
2. `rule_id`
3. `risk_level`
4. `action`（allow/deny/escalate）
5. `reason`
6. `request_id`
7. `resolver`（human/agent/policy）
8. `resolved_at`

## 与 Sandbox 的关系

1. Policy 决定“应不应该做”。
2. Sandbox 约束“能不能做到”。

原则：Policy 永远不能扩大 sandbox 物理边界，只能收紧或触发人工接管。

## 与 Outcome Ownership 的关系

HITE 下人类不是“按钮操作员”，而是“结果 owner”。

治理策略因此要支持：

1. 在任务开始前定义边界与预算。
2. 在异常时最小必要介入。
3. 事后依据审计链对结果负责。

## 最小落地路径（建议）

1. 先定义 10-20 条高价值边界规则（生产、资金、删除、推送）。
2. 所有边界命中统一走 `Yield/Resume`。
3. rollout 增补治理审计字段。
4. 为边界策略建立回归测试场景。

## 验收要点

1. 高风险动作不会在无人确认下越界执行。
2. 低风险动作不被过度阻塞（避免审批疲劳）。
3. 每次边界决策都可追踪到规则、原因与最终责任归属。
