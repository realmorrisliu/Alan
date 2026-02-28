# Alan 文档索引

## 如何阅读

为避免“理念文章”和“可执行规范”混淆，文档按优先级分层：

1. **内核/执行规范（最高优先级）**：约束状态机、边界、不变量。
2. **主线架构规范**：描述当前系统结构与职责分工。
3. **设计 RFC（迁移解释）**：说明为何改、如何迁移。
4. **验证体系文档**：定义协议与系统级回归方法。
5. **理念文章**：提供方向，不作为协议真值源。

---

## 1) 内核/执行规范（最高优先级）

- [`spec/kernel_contract.md`](./spec/kernel_contract.md)：内核不变量与职责边界
- [`spec/execution_model.md`](./spec/execution_model.md)：Task/Run/Session/Turn 执行模型与状态机
- [`spec/memory_architecture.md`](./spec/memory_architecture.md)：L0/L1/L2 memory 分层与写入/检索契约
- [`spec/compaction_contract.md`](./spec/compaction_contract.md)：compaction 触发、输出、质量与退化策略
- [`spec/app_server_protocol.md`](./spec/app_server_protocol.md)：线程/轮次协议抽象与当前 API 映射
- [`spec/governance_boundaries.md`](./spec/governance_boundaries.md)：提交边界（Commit Boundaries）与审计要求
- [`spec/scheduler_contract.md`](./spec/scheduler_contract.md)：定时、休眠、唤醒与重启恢复语义
- [`spec/interaction_inbox_contract.md`](./spec/interaction_inbox_contract.md)：`steer/follow_up/next_turn` 输入语义与队列规则
- [`spec/durable_run_contract.md`](./spec/durable_run_contract.md)：checkpoint、幂等、副作用恢复契约
- [`spec/extension_contract.md`](./spec/extension_contract.md)：extension/plugin 生命周期、权限与能力声明契约
- [`spec/capability_router.md`](./spec/capability_router.md)：builtin/extension/bridge 统一能力路由契约
- [`spec/harness_bridge.md`](./spec/harness_bridge.md)：远程节点桥接、恢复、鉴权与审计契约

---

## 2) 主线架构规范（目标 + 当前）

- [`architecture.md`](./architecture.md)：三层抽象、crate 分工、运行时结构
- [`autonomy_layered_design.md`](./autonomy_layered_design.md)：自治能力分层设计（runtime/daemon/skills/harness 边界与落地路径）
- [`policy_over_sandbox.md`](./policy_over_sandbox.md)：V2 工具治理规范（Policy 决策 + Sandbox 执行边界，breaking 迁移中）
- [`skills_and_tools.md`](./skills_and_tools.md)：Tool/Skill 机制、作用域、沙箱与策略

协议真值源代码：

- `crates/protocol/src/op.rs`
- `crates/protocol/src/event.rs`
- `crates/runtime/src/tape.rs`
- `crates/runtime/src/rollout.rs`

---

## 3) 设计 RFC（迁移解释）

- [`alphabet_design.md`](./alphabet_design.md)：Alphabet 分层设计与迁移动机

说明：该文档同时包含“迁移前背景”和“目标模型”，阅读时请关注状态标注。

---

## 4) 验证体系文档

- [`testing_strategy.md`](./testing_strategy.md)：协议一致性测试策略与 CI 建议
- [`harness/README.md`](./harness/README.md)：系统级长程回归（loop、governance、compaction、memory、autonomy、self_eval）

---

## 5) 理念文章

- [`human_in_the_end.md`](./human_in_the_end.md)：HITE 设计哲学与行业背景

说明：理念文档用于阐释方向，落实请以 `spec/` 下规范为准。
