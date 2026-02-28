# Alan 文档索引

## 如何阅读

为避免“历史设计稿”和“目标规范”混淆，文档分为三类：

1. **规范文档（目标 + 当前）**：定义主线架构与协议方向。部分章节会标注“已实现”或“迁移中”。
2. **设计 RFC（迁移解释）**：解释为什么这么改、怎么演进。
3. **理念文章**：阐述设计哲学，不作为协议真值源。

---

## 1) 规范文档（目标 + 当前）

- [`architecture.md`](./architecture.md)：三层抽象、crate 分工、运行时结构
- [`policy_over_sandbox.md`](./policy_over_sandbox.md)：V2 工具治理规范（Policy 决策 + Sandbox 执行边界，breaking 迁移中）
- [`skills_and_tools.md`](./skills_and_tools.md)：Tool/Skill 机制、作用域、沙箱与策略
- [`testing_strategy.md`](./testing_strategy.md)：协议一致性测试策略与 CI 建议

协议真值源代码：

- `crates/protocol/src/op.rs`
- `crates/protocol/src/event.rs`

---

## 2) 设计 RFC（迁移解释）

- [`alphabet_design.md`](./alphabet_design.md)：Alphabet 分层设计与迁移动机

说明：本文件同时包含“迁移前背景”和“目标模型”，阅读时请关注其状态标注。

---

## 3) 理念文章

- [`human_in_the_end.md`](./human_in_the_end.md)：HITE 设计哲学与行业背景

---

## 4) 演讲材料

- `agent-evolution-presentation-outline.md`
- `agent-evolution-speaker-notes.md`

这两份属于演讲辅助材料，不作为实现规范。
