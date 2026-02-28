# 测试策略：避免客户端-服务端协议漂移

## 目标

Alan 的事件流是前后端协作的契约。测试策略的核心目标是：

1. 服务端发出的事件，客户端必须能消费和渲染。
2. 协议演进时，兼容层有明确边界，不靠隐式约定。
3. CI 能尽早发现“协议改了但客户端没跟上”。

---

## 当前协议基线（2026-02）

以 `alan_protocol::Event` / `alan_protocol::Op` 为准：

- Event：`turn_started`、`turn_completed`、`text_delta`、`thinking_delta`、`tool_call_started`、`tool_call_completed`、`yield`、`error`
- Op：`turn`、`input`、`resume`、`interrupt`、`register_dynamic_tools`、`compact`、`rollback`

参考实现：
- `crates/protocol/src/event.rs`
- `crates/protocol/src/op.rs`

---

## 测试分层

### 1) 契约测试（Contract）

文件：`crates/alan/tests/event_contract_test.rs`

作用：
- 验证客户端视角的最小可见性契约（例如文本回复至少要收到可展示的内容）。
- 验证工具调用事件在前端可被识别。
- 验证空响应时的回退消息契约。

这层不关心内部实现细节，只关心“用户最终能否看到正确结果”。

### 2) 事件序列验证（Sequence Validation）

文件：`crates/alan/tests/event_sequence_validation_test.rs`

作用：
- 验证不同场景下事件序列的相对顺序与必需项。
- 覆盖文本回复、工具调用、空响应回退等典型流程。

示例模式（当前）：

```rust
let expected_sequence = vec![
    EventPattern::new("turn_started").required(),
    EventPattern::new("thinking_delta").required(),
    EventPattern::new("text_delta").required(),
    EventPattern::new("turn_completed").required(),
];
```

### 3) 集成事件流测试（Integration）

文件：`crates/alan/tests/integration_event_flow_test.rs`

作用：
- 验证 `EventEnvelope` 基础属性（如时间戳单调性）。
- 验证流式事件在 transport 包装层的基本稳定性。

---

## 类型共享与兼容策略

脚本：`scripts/generate-ts-types.sh`

产物：
- `clients/tui/src/generated/types.ts`
- `clients/tui/src/generated/event-map.ts`

说明：
- 生成类型包含“协议核心事件 + 客户端兼容事件集合”。
- 因此 TypeScript 的 `EventType` 可能是协议的超集；协议真值仍以 Rust `alan_protocol` 为准。
- 客户端可保留兼容分支（例如历史字段），但新功能应优先对齐 `text_delta` / `thinking_delta` / `yield` 等当前事件。

---

## 变更流程建议

### 新增或修改事件时

1. 先改 `crates/protocol/src/event.rs`（协议源头）。
2. 更新契约测试：`crates/alan/tests/event_contract_test.rs`。
3. 更新序列测试：`crates/alan/tests/event_sequence_validation_test.rs`。
4. 更新客户端处理逻辑（TUI / ask / Apple）。
5. 运行类型生成：`./scripts/generate-ts-types.sh`。
6. 运行测试：`cargo test --workspace`。

### 新增或修改 Op 时

1. 先改 `crates/protocol/src/op.rs`。
2. 更新 daemon 路由/提交处理相关测试。
3. 更新客户端提交负载。
4. 运行全量测试与类型生成。

---

## CI 建议

```yaml
- name: Run contract tests
  run: cargo test -p alan --test event_contract_test

- name: Run event sequence tests
  run: cargo test -p alan --test event_sequence_validation_test

- name: Verify generated TS types are up to date
  run: |
    ./scripts/generate-ts-types.sh
    git diff --exit-code clients/tui/src/generated/
```

---

## 总结

避免协议不匹配的关键是“先定义契约，再实现兼容”：

1. 协议真值源：`alan_protocol`
2. 行为真值源：契约测试 + 序列测试
3. 前端同步机制：生成类型 + CI 校验
