# Compaction Contract

> Status: VNext contract（兼容当前 `compact` 能力，补全触发/质量/审计语义）。

## 目标

Compaction 不是“删历史”，而是“在有界上下文下保持执行连续性”的核心机制。

必须保证：

1. 减少上下文占用。
2. 保留关键决策与未完成事项。
3. 不破坏后续执行可恢复性。

## 触发类型

### 1) 手动触发

- 由显式操作触发（例如 `Op::Compact`）。
- 可携带聚焦指令（例如“重点保留待办与约束”）。

### 2) 自动触发

- 在接近上下文窗口上限时触发。
- 建议使用双阈值：
  - `hard_threshold`：必须压缩
  - `soft_threshold`：先执行 pre-compaction memory flush

## 输入范围契约

压缩输入应包含：

1. 当前会话历史（可用于后续推理的消息、工具结果、关键系统片段）。
2. 当前策略与上下文边界（必要时作为不可丢失信息）。

压缩输入不应包含：

1. 与当前任务无关的大体量冗余工具输出（可先裁剪）。
2. 无法安全再利用的噪声日志。

## 输出契约

压缩后会话至少包含：

1. **Compaction summary item**（结构化摘要条目）。
2. **Recent window**（最近关键消息保持原样）。
3. **Reference marker**（可选）：标识摘要覆盖区间与来源。

摘要最低要求：

1. 关键决策。
2. 当前约束。
3. 未完成事项与下一步。
4. 关键标识符（ID、路径、命令上下文）不失真。

## 质量约束

1. **信息安全**：不得注入不存在的事实。
2. **标识符保真**：ID/路径/哈希等不可随意改写。
3. **可执行性**：摘要应能直接支持下一轮动作选择。

## 与 Memory 的协同

建议在自动 compaction 前执行一次 pre-compaction flush：

1. 把高价值长期信息写入 L1 memory。
2. flush 回合默认静默。
3. flush 失败记录告警，不阻塞 compaction 主流程。

## 事件与审计字段

每次 compaction 至少应落盘：

1. `trigger`（manual/auto）
2. `reason`（window pressure / explicit request）
3. `input_size` / `output_size`
4. `summary_id` 或等价引用
5. `duration_ms`
6. `result`（success/failure/retry）

若触发自动重试，应记录 `retry_count` 与失败原因。

## 失败退化策略

1. **摘要失败**：保留原上下文并返回可恢复错误；禁止静默清空。
2. **部分失败**：可降级为“仅裁剪大工具输出 + 保留最近窗口”。
3. **连续失败**：必须显式告警并建议新建 session/run。

## 幂等与重入

1. 对同一输入快照重复 compaction，输出应语义等价。
2. 避免在同一 turn 内无限 compaction 循环。
3. compaction 过程应可中断并保持会话一致。

## 与 Rollback/Fork 的关系

1. rollback 必须识别 compaction 边界，避免破坏摘要一致性。
2. fork 后需继承必要摘要上下文，保证分支可继续执行。

## 验收要点

1. 压缩后 token 占用显著下降且行为连续。
2. 摘要内容覆盖“决策/约束/待办/关键标识符”。
3. 审计日志可完整还原 compaction 的因果链。
