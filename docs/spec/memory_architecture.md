# Memory Architecture

> Status: VNext contract (基于当前 Tape + Workspace Memory 能力演进)。

## 目标

把“模型短期上下文”与“长期可持久知识”解耦，形成可解释、可维护、可审计的 memory 体系。

Alan memory 的核心原则：

1. **文件是事实真值源**，不是模型内部隐式记忆。
2. **检索是能力层，不是状态层**。
3. **写入要有策略，不依赖运气**。

## 三层记忆模型

### L0: Execution Memory（执行记忆）

- 载体：`Tape + Rollout`
- 生命周期：Session 级
- 用途：保证当前任务执行连续性
- 特征：高保真、易膨胀、需要 compaction

### L1: Workspace Memory（工作区长期记忆）

- 载体：`{workspace}/.alan/memory/`
- 生命周期：Workspace 级
- 用途：沉淀稳定偏好、决策、约束、关键事实
- 特征：人可读、可编辑、可版本化

建议基础文件：

- `MEMORY.md`：长期稳定记忆（规则、偏好、长期背景）
- `memory/YYYY-MM-DD.md`：日记型增量记录

### L2: Retrieval Memory（检索索引层，可选）

- 载体：向量/混合索引（可插拔）
- 生命周期：可重建
- 用途：语义召回，提升跨天检索效率
- 特征：缓存层，不是事实源，随时可重建

## 当前实现映射

目前 Alan 已具备：

1. L0：`Tape` 与 `rollout` 持久化。
2. L1（基础）：workspace 下 memory 目录与 memory skill。

仍缺：

1. 统一 memory 工具契约（如 search/get）。
2. compaction 前自动 memory flush 策略。
3. L2 索引层规范与后端接口。

## 写入策略契约

### 何时写入 L1

1. 用户明确要求“记住”。
2. 出现可复用决策（规则、约束、偏好）。
3. 即将触发 compaction 且存在高价值未沉淀信息（pre-compaction flush）。

### 不应写入

1. 短期噪声。
2. 可直接从源码/系统实时读取的易变事实。
3. 敏感数据（除非明确允许并有治理策略）。

## 读取策略契约

1. 先判定问题是否需要长期记忆。
2. 检索遵循“先窄后宽”：
  - 先精确文件读取（MEMORY.md / 当日日志）
  - 再语义搜索（L2）
3. 读取结果应携带来源路径，便于审计与追责。

## 与 Compaction 的联动

### 预压缩 memory flush（建议）

在达到 compaction 软阈值前触发一次静默回合：

1. 提醒 agent 把高价值信息写入 L1。
2. 默认不产生用户可见回复（除非确有必要）。
3. 同一 compaction 周期只触发一次。

### 合同要求

1. flush 失败不应阻塞主流程，但要记录失败事件。
2. flush 跳过条件需显式（例如 workspace 只读）。

## 数据治理与审计

1. 每条 memory 写入建议包含：`who/when/why/source`。
2. 可选字段：可信度、过期时间、敏感级别。
3. 删除/改写 memory 要有可追踪记录。

## 索引层（L2）抽象接口（草案）

```text
index.upsert(path, content, metadata)
index.delete(path)
index.search(query, options) -> snippets[]
index.read(path, range) -> text
```

要求：

1. 索引失效不影响 L1 文件读写。
2. 后端可替换（sqlite/vector/hybrid）。
3. 检索结果必须能回链到 L1 原文。

## 验收要点

1. L0/L1/L2 职责清晰，不互相挤压。
2. compaction 后关键上下文可通过 L1 恢复。
3. 记忆写入与检索链路具备可审计性。
