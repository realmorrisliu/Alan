# Alan Harness

> Status: VNext validation framework blueprint.

## 目标

Harness 是 Alan 的系统级验证框架，不是单个 crate 的单元测试集合。

它关注的问题是：

1. 长时间运行是否稳定。
2. 复杂工具链与策略边界下是否可控。
3. compaction / rollback / recovery 后行为是否连续。
4. 协议与多客户端集成是否不漂移。

## 为什么需要 Harness

仅依赖 unit/integration 测试无法覆盖：

1. 多轮 tool-call 循环中的状态漂移。
2. 上下文膨胀后的压缩退化。
3. 断线重连与事件补偿缺口。
4. 策略边界命中时的人机接管路径。

Harness 的目标是把这些“运行时真实风险”变成可回归场景。

## 场景分层

### 1) Protocol Conformance

- 输入 Op 序列与输出 Event 序列一致性。
- 重点：turn 边界、yield/resume、interrupt、events/read gap 行为。

### 2) Loop Stability

- 长工具链回合（10+ tool loops）。
- steering 插入、中断恢复、超时重试。
- 目标：无死循环、无重复副作用、无状态悬挂。

### 3) Governance Boundaries

- allow/deny/escalate 命中验证。
- 关键提交边界必须触发人工接管路径。

### 4) Compaction Robustness

- 自动/手动 compaction 后连续执行。
- 摘要保真与关键待办保留验证。

### 5) Memory Durability

- memory 写入、读取、跨 session 恢复。
- pre-compaction memory flush 行为验证（启用后）。

### 6) Replay & Rollback

- 回放不重复副作用。
- rollback 后事件与状态一致。

### 7) Autonomy (Scheduler & Recovery)

- 定时触发、sleep/wake、重启恢复链路验证。
- 关注点：任务不丢失、重复分发不重复副作用、到点执行误差可控。

### 8) Self-Eval (Prompt/Profile Governance)

- 候选 prompt/profile 的离线对比评测。
- 关注点：成功率提升是否伴随成本、风险或越界回归。

## 统一产物（Artifacts）

每个 harness 场景建议产出：

1. 输入脚本（Op 序列）。
2. 事件轨迹（Event JSONL）。
3. 决策轨迹（policy/sandbox/tool trace）。
4. 断言报告（pass/fail + diff）。

## 关键指标（KPI）

1. Turn 成功率与中断恢复率。
2. 平均工具回合数与失败分布。
3. Compaction 触发率与 compaction 后失败率。
4. Escalation 命中率与人工解决时延。
5. Event gap 检测率与恢复成功率。

## 建议落地顺序

1. 先做协议与生命周期基线（Protocol + Loop）。
2. 再做治理边界与 compaction 回归。
3. 再补 memory durability 与 replay/rollback 套件。
4. 最后引入 autonomy 与 self_eval 发布门禁。

## 与现有测试关系

- `docs/testing_strategy.md`：定义协议真值源与基础契约测试。
- Harness：在其上补系统级、长程、异常路径验证。

两者关系：

1. 契约测试保证“接口不漂移”。
2. Harness 保证“系统在真实压力下可工作”。

## 目录建议（后续）

```text
docs/harness/
  README.md
  scenarios/
    protocol/
    loop/
    governance/
    compaction/
    memory/
    replay/
    autonomy/
    self_eval/
  metrics/
    kpi.md
```

## 可执行场景矩阵（MVP）

建议先落一批可自动运行的场景（每个场景必须有输入脚本、断言、产物）：

1. `protocol/input_modes`
   - 目标：验证 `steer/follow_up/next_turn` 协议与队列语义。
   - 断言：输入应用顺序、队列上限、drop 行为可观测。
2. `loop/steer_during_tool_batch`
   - 目标：验证工具批次中 steer 中断与剩余 tool 跳过语义。
   - 断言：跳过标记、后续重规划、turn 一致性。
3. `autonomy/scheduler_wake`
   - 目标：验证 `sleep_until/schedule_at` 到点触发。
   - 断言：触发时间、run 状态切换、审计字段完整。
4. `autonomy/reboot_resume`
   - 目标：验证 daemon 重启后 run 恢复。
   - 断言：未终态 run 可恢复，checkpoint 连续。
5. `autonomy/dedup_side_effect`
   - 目标：验证重复分发下副作用去重。
   - 断言：同 idempotency key 不重复执行不可逆动作。
6. `governance/recovery_boundary`
   - 目标：验证恢复路径同样命中高风险边界。
   - 断言：无自动越界执行，yield/resume 可追溯。
7. `self_eval/profile_regression`
   - 目标：对比 baseline/candidate prompt profile。
   - 断言：通过阈值（成功率、成本、越界率）才可晋升。

## 发布门禁建议

将以下场景设为发布阻断（blocking）：

1. `protocol/input_modes`
2. `autonomy/reboot_resume`
3. `autonomy/dedup_side_effect`
4. `governance/recovery_boundary`
5. `self_eval/profile_regression`

## 验收要点

1. 关键回归场景可重复执行。
2. 失败可定位到具体环节（协议/策略/工具/压缩）。
3. Harness 结果可作为发布门禁输入。
