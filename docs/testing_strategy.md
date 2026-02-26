# 测试策略 - 避免客户端-服务端协议不匹配

## 问题回顾

之前出现的问题：服务端发送了 `MessageDeltaChunk` 事件，但客户端（TUI/ask）只处理 `MessageDelta` 事件，导致用户看不到 LLM 返回的消息。

根本原因：**缺少对客户端-服务端协议一致性的验证**。

## 测试金字塔

```
       /\
      /  \
     / E2E \     <- 端到端测试（验证完整流程）
    /--------\
   /          \
  / Integration \  <- 集成测试（验证事件流）
 /----------------\
/                  \
/   Contract Tests   \ <- 契约测试（验证协议一致性）
/----------------------\
/                        \
/      Unit Tests          \ <- 单元测试（验证单个组件）
/----------------------------\
```

## 测试类型

### 1. 契约测试（Contract Tests）

**目的**：确保服务端发送的事件能被客户端正确处理

**文件**：`crates/alan/tests/event_contract_test.rs`

**核心测试**：
- `contract_text_response_must_emit_displayable_event`：验证文本响应时客户端能收到可显示的消息
- `contract_tool_call_must_emit_tool_events`：验证工具调用事件正确传递
- `contract_empty_response_must_show_fallback_message`：验证空响应回退机制
- `contract_turn_must_emit_complete_event_sequence`：验证完整的事件序列

**关键断言**：
```rust
// 契约：客户端必须能显示消息给用户
assert!(
    client.has_received_message(),
    "Contract violation: Client must receive at least one displayable message event \
     when LLM returns text response."
);
```

### 2. 集成测试（Integration Tests）

**目的**：验证端到端的事件流

**文件**：`crates/alan/tests/integration_event_flow_test.rs`

**核心功能**：
- 事件序列模式匹配
- 关键事件属性验证（event_id、时间戳顺序等）
- TurnStarted/TurnCompleted 平衡检查

**使用方式**：
```rust
assert_event_sequence(&events, vec![
    EventPattern::TurnStarted,
    EventPattern::Thinking,
    EventPattern::ThinkingComplete,
    EventPattern::MessageDelta,
    EventPattern::TaskCompleted,
    EventPattern::TurnCompleted,
]);
```

### 3. 事件序列验证测试

**目的**：验证各种场景下的事件序列

**文件**：`crates/alan/tests/event_sequence_validation_test.rs`

**测试场景**：
- `sequence_text_response`：正常文本响应
- `sequence_tool_call_response`：工具调用响应
- `sequence_empty_response_fallback`：空响应回退

**模式定义**：
```rust
let expected_sequence = vec![
    EventPattern::new("turn_started").required(),
    EventPattern::new("thinking").required(),
    EventPattern::new("message_delta").required(),
    EventPattern::new("turn_completed").required(),
];
```

### 4. 类型共享（Type Sharing）

**目的**：确保客户端和服务端使用一致的类型定义

**脚本**：`scripts/generate-ts-types.sh`

**生成了**：
- `clients/tui/src/generated/types.ts`：TypeScript 类型定义
- `clients/tui/src/generated/event-map.ts`：事件处理器映射

**关键类型**：
```typescript
export interface MessageDeltaEvent extends BaseEvent {
  type: 'message_delta';
  content: string;
}

export interface MessageDeltaChunkEvent extends BaseEvent {
  type: 'message_delta_chunk';
  chunk: string;
  is_final: boolean;
}
```

## 为什么现有测试没有覆盖到这个问题？

### 1. **单元测试的局限性**

现有测试是单元测试，只验证单个组件：

```rust
// turn_executor.rs 中的测试
#[tokio::test]
async fn test_run_turn_with_content_response() {
    // ...
    let has_turn_started = events.iter().any(|e| matches!(e, Event::TurnStarted {}));
    // ...
    // 问题：只验证了服务端发送了事件，没有验证客户端能处理
}
```

**缺失**：没有验证客户端是否订阅了这些事件类型。

### 2. **没有事件序列契约**

没有测试定义"一个完整的 turn 必须包含哪些事件"。

### 3. **Mock 数据不一致**

测试使用的 Mock 数据可能和实际运行时不一致。

### 4. **没有端到端验证**

没有自动化测试验证"用户提交消息 -> 看到响应"的完整流程。

## 推荐的开发流程

### 添加新事件类型时：

1. **在 Rust 中定义事件**（`crates/protocol/src/event.rs`）
2. **添加契约测试**（`crates/alan/tests/event_contract_test.rs`）
3. **更新类型生成脚本**（`scripts/generate-ts-types.sh`）
4. **生成 TypeScript 类型**：`./scripts/generate-ts-types.sh`
5. **在客户端实现事件处理器**
6. **添加集成测试**验证完整流程

### 修改现有事件时：

1. **检查契约测试**：确保测试会失败，提示需要更新客户端
2. **同时修改服务端和客户端**
3. **运行所有测试**：`cargo test --workspace`
4. **运行契约测试**：`cargo test -p alan --test event_contract_test`

## CI/CD 集成

建议在 CI 中添加：

```yaml
# .github/workflows/test.yml
- name: Run contract tests
  run: cargo test -p alan --test event_contract_test

- name: Run event sequence tests
  run: cargo test -p alan --test event_sequence_validation_test

- name: Verify TypeScript types are up to date
  run: |
    ./scripts/generate-ts-types.sh
    git diff --exit-code clients/tui/src/generated/
```

## 未来改进

### 1. 自动化契约测试生成

从 Rust 类型自动生成契约测试框架。

### 2. 协议版本控制

添加协议版本号，当不兼容变更时：
- 服务端检测客户端版本
- 客户端检测服务端版本
- 不匹配时给出清晰的错误信息

### 3. 事件使用分析

静态分析工具检查：
- 哪些事件类型服务端会发送
- 哪些事件类型客户端处理了
- 是否有不匹配的情况

### 4. 端到端测试套件

使用真实（或 mock）LLM 的完整测试：
```rust
#[tokio::test]
async fn e2e_user_asks_question_gets_response() {
    let app = TestApp::new().await;
    let session = app.create_session().await;
    
    let events = app
        .send_message_and_wait_for_response("Hello")
        .await;
    
    // 验证用户看到了响应
    let displayed_messages = app.get_displayed_messages();
    assert!(!displayed_messages.is_empty());
}
```

## 总结

避免协议不匹配的关键是：**在服务端和客户端之间建立显式契约**。

通过：
1. **契约测试** - 验证双方对协议的理解一致
2. **类型共享** - 确保类型定义同步
3. **集成测试** - 验证完整流程
4. **CI 检查** - 自动化验证

可以有效防止类似问题再次发生。
