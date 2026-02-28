# Alan 自治能力分层设计（Autonomy Layered Architecture）

> Status: VNext 设计文档（用于统一抽象边界与落地顺序）。  
> 范围：解释“哪些能力应放在 runtime / daemon / skills / harness”，并与 Alan 现有哲学与协议对齐。

## 1. 背景与问题

Alan 当前已经具备：

1. 基础 turn 状态机、yield/resume、tool orchestration。
2. 会话持久化与 daemon 重启后的 session 恢复。
3. policy-over-sandbox 的治理路径。
4. steering 输入在工具批次中的中断能力。

但当目标升级为「通用 long-running agent 基座」时，会出现新的系统性需求：

1. Agent 能自己设定“未来执行点”（提醒、定时任务、睡眠唤醒）。
2. Agent 在系统重启后可继续未完成任务，而不是只恢复会话壳。
3. 人类输入与 agent 执行并行共存，不再被“当前 turn 串行队列”完全阻塞。
4. prompt/策略优化进入可验证闭环（harness），而不是线上即兴漂移。

这些需求本质上是“可靠性与抽象边界”问题，不是单一 prompt 或 skill 可以稳定解决的问题。

## 2. 设计目标

本设计目标是：

1. 明确分层职责，让内核保持小而稳。
2. 把“可恢复、可审计、可幂等”的能力沉到系统层。
3. 把“可替换、可演进”的业务流程留在 skill/tool 层。
4. 把“是否有效”交给 harness 做离线评估与回归门禁。

非目标：

1. 不把 runtime 变成业务工作流引擎。
2. 不让 skill 承担系统级可靠性（定时、重启恢复、幂等保障）。
3. 不在生产路径中直接做自动 prompt 自我修改并立即生效。

## 3. Alan 哲学对齐

本设计遵循 Alan 既有哲学与文档基线：

1. **AI Turing Machine**：runtime 负责状态推进，不承载业务语义（`docs/architecture.md`）。
2. **Kernel 小而稳**：不变量与状态机优先（`docs/spec/kernel_contract.md`）。
3. **Skills-first + Extensions-ready**：skills 负责流程编排，extensions 负责可插拔能力实现，工具保留原子副作用语义。
4. **Human-in-the-End**：人类是结果 owner，介入聚焦边界与异常（`docs/human_in_the_end.md`）。
5. **Policy over Sandbox**：策略决定应不应该做，沙箱约束能不能做（`docs/policy_over_sandbox.md`）。
6. **Unix 哲学**：内核机制尽量通用，业务流程通过小工具与文本技能组合。

## 4. 分层判定规则

任何新能力先用以下规则判定层级：

1. 需要“确定性 + 持久化 + 崩溃恢复”的能力，放 `daemon/runtime`。
2. 需要“业务流程可替换”的能力，放 `skills`。
3. 需要“外部副作用执行”的能力，放 `tools`。
4. 需要“效果验证/质量评估”的能力，放 `harness`（验证层，不承载生产逻辑）。

## 5. 分层架构总览

### L0: Protocol（协议层）

职责：

1. 定义输入输出语义（`Op` / `Event`）。
2. 为多客户端提供稳定契约。
3. 明确 turn、yield、resume、steer 等控制面语义。

参考：`crates/protocol/src/op.rs`、`crates/protocol/src/event.rs`、`docs/spec/app_server_protocol.md`。

### L1: Runtime Kernel（执行内核）

职责：

1. turn 状态机与 tool loop。
2. tape/rollout 真值与执行一致性。
3. policy 决策接入、yield/resume 对称恢复。
4. checkpoint 原语与幂等语义（面向 run 恢复）。

不负责：

1. 跨天调度。
2. 产品级队列/提醒策略。
3. 具体业务流程 DSL。

### L2: Daemon/Host（编排与托管层）

职责：

1. runtime 生命周期管理（启动/恢复/重连）。
2. 持久任务队列与调度器（定时、重试、睡眠唤醒）。
3. 任务级对象管理（Task/Run 元数据）。
4. 输入收件箱分流（steer/follow_up/next_turn）。

### L3: Skills + Tools（能力层）

职责：

1. skill 定义业务流程（如何分解任务、如何调用工具）。
2. tool 实现外部动作（文件/命令/网络/API）。
3. 在内核约束内实现可替换能力。

边界：

1. skill 不负责系统恢复与幂等保障。
2. tool 不绕过 policy/sandbox。

### L4: Harness（验证层）

职责：

1. 系统级场景回归（长运行、恢复、边界、安全）。
2. prompt/profile 评估与晋升门禁。
3. 指标化比较（成功率、成本、越界率、恢复率）。

## 6. 能力到层级的映射

| 能力 | 主实现层 | 说明 |
| --- | --- | --- |
| 定时提醒 / 到点执行 | L2 Daemon | 需要持久调度队列与重启恢复 |
| 空闲休眠 / 唤醒 | L2 Daemon + L1 Runtime | daemon 决定唤醒时机，runtime 保证恢复语义 |
| 系统重启后继续工作 | L2 + L1 | daemon 重建运行面，runtime 从 checkpoint 继续 |
| 人类消息并行输入 | L0 + L2 + L1 | 协议定义语义，daemon 收件箱分流，runtime 消费执行 |
| feature2 提前影响 feature1 设计 | L2 Inbox + L1 planning hook | 将 follow-up 预览注入当前规划上下文 |
| 自举（自改代码/自重启/续跑） | L1/L2 机制 + L3 skill 流程 | 机制内建，流程可由 skill 编排 |
| Prompt 级自评估优化 | L4 Harness | 离线评估后晋升，不直接在线自改 |

## 7. 核心对象模型（Task / Run / Session / Turn）

在现有 Session/Turn 上补齐 Task/Run 维度：

1. **Task**：业务目标、约束、owner、SLA。
2. **Run**：一次执行尝试，可重试、可中断恢复。
3. **Session**：Run 的当前上下文窗口容器。
4. **Turn**：最小状态推进单元。

建议的 Run 状态：

1. `pending`
2. `running`
3. `sleeping`（等待时间/外部事件）
4. `yielded`（等待人工/结构化输入）
5. `completed`
6. `failed`
7. `cancelled`

关键原则：

1. `sleeping` 与 `yielded` 可跨进程、跨重启恢复。
2. Session 可轮换，Run 连续。
3. Task 是 owner-facing 对象，Run 是系统执行对象。

## 8. 三条关键执行链路

### 8.1 持久调度链路（Reminder / Sleep / Wake）

最小能力：

1. `schedule_at(run_id, wake_at, payload)`
2. `sleep_until(run_id, wake_at)`
3. `retry_with_backoff(run_id, policy)`
4. `on_boot_resume()`（daemon 启动时扫描并恢复到期/未完成 run）

存储要求：

1. 任务记录持久化（可先 JSON/SQLite，后续可插拔）。
2. 记录 last_checkpoint_id 与 next_wake_at。
3. 对同一 run 唤醒具幂等保护（避免重复执行）。

### 8.2 并行输入链路（Human IO / Agent IO）

输入语义拆分为三类：

1. `steer`：高优先级，当前执行中断点注入。
2. `follow_up`：当前执行完成后立即处理。
3. `next_turn`：仅作为下一轮用户回合上下文。

关键行为：

1. tool batch 期间收到 steer，可跳过剩余可跳过工具并重规划。
2. follow_up 不阻塞当前执行，但可参与“未来意图预览”。
3. runtime 内保持 turn 一致性，daemon 负责收件箱优先级与队列语义。

### 8.3 重启续跑链路（Durable Run）

checkpoint 建议至少包含：

1. `run_id/task_id/session_id`
2. 当前 turn 状态与 pending yield
3. 最近已确认副作用（带 idempotency key）
4. 下一步执行意图与恢复入口

恢复流程：

1. daemon 启动加载未终态 run。
2. 对 `running/sleeping/yielded` run 进行状态归一化。
3. runtime 从 checkpoint 恢复；对副作用调用按幂等键去重。

## 9. 自举能力的抽象边界

以“自动更新系统并重启后验证”与“自改代码后重启继续”为例：

1. 是否允许执行此类动作由 governance boundary 决定（L1+policy）。
2. 如何执行（命令、检查、重试）由 skill 编排（L3）。
3. 如何跨重启连续由 durable run + scheduler 保障（L2/L1）。

因此：

1. “能安全续跑”是系统能力。
2. “做什么续跑”是技能策略。

## 10. 自评估与 prompt 进化放入 Harness

目标不是在线自发漂移，而是可验证演进：

1. 维护 `prompt_profile`（如 baseline/candidate）。
2. Harness 跑固定场景集（协议、恢复、边界、长任务）。
3. 产出指标：成功率、成本、越界率、恢复成功率、重复副作用率。
4. 达到阈值才允许 candidate 晋升为默认 profile。

建议新增 harness 套件：

1. `autonomy/scheduler_recovery`
2. `autonomy/parallel_input_semantics`
3. `autonomy/reboot_continuation`
4. `autonomy/prompt_profile_regression`

## 11. 与现有 Alan 代码基线的衔接

当前已可复用基础：

1. `turn_driver`/`turn_executor`/`tool_orchestrator`：turn 执行、steering、yield/resume。
2. `session_store` + `AppState::ensure_sessions_recovered`：daemon 重启后的 session 恢复壳。
3. `policy` + `sandbox` + `approval`：边界决策与人工接管机制。

需要新增的核心模块（建议）：

1. `daemon/task_store`：Task/Run 持久化。
2. `daemon/scheduler`：定时与唤醒执行器。
3. `runtime/checkpoint`：run-level checkpoint 与恢复接口。
4. `protocol` 扩展：显式输入投递模式与 run/task 元数据字段。

## 12. 分阶段落地计划

### Phase 1: 协议与对象引入（兼容优先）

1. 在 metadata 中引入 `task_id/run_id`（可选）。
2. 定义输入模式（steer/follow_up/next_turn）但先做服务端兼容映射。
3. 保持现有 `/sessions` API 可用。

### Phase 2: 持久调度最小闭环

1. 实现 `task_store + scheduler`。
2. 支持 `schedule_at/sleep_until/on_boot_resume`。
3. 完成最小 e2e：定时唤醒 -> 执行 -> 产出事件。

### Phase 3: Durable Run 与重启续跑

1. 引入 checkpoint 与幂等键。
2. 支持重启后 run 自动恢复。
3. 覆盖“外部副作用不重复”测试。

### Phase 4: Harness 评测闭环

1. 增加 autonomy 套件。
2. 引入 prompt profile 评测与晋升规则。
3. 将关键指标接入发布门禁。

## 13. 验收标准

1. 系统重启后，未终态 run 能自动恢复到可执行状态。
2. 同一高风险动作不会因恢复/重试重复副作用。
3. 人类输入在执行中可并行注入且语义稳定。
4. 定时任务在时序误差可控范围内触发并可审计。
5. prompt/profile 演进必须经过 harness 指标门禁。
