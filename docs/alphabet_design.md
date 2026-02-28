# Alphabet Design: 分层字母表

在 Alan AI Turing Machine 的架构中，**Alphabet (字母表)** 定义了机器能够读取和写入磁带 (Tape) 的符号集合，以及它与外部世界进行通信的 I/O 信号。

本文档是 Alan 系统中 `Message`（内部磁带符号）、`Op`（外部输入指令）和 `Event`（外部输出事件）的重新设计规范。核心原则：**区分内容与动作，收敛控制流，保留必要的类型区分**。

---

## 文档定位

这是一份“迁移 RFC + 设计解释”文档，包含两类内容：

- **历史背景**：解释迁移前为什么会复杂（用于理解设计动机）
- **目标模型**：解释迁移后的统一抽象（用于指导后续演进）

若你只关心“当前线上的协议真值”，请以以下代码为准：

- `crates/protocol/src/op.rs`
- `crates/protocol/src/event.rs`
- `crates/runtime/src/tape.rs`

---

## 1. 迁移前现状（历史背景）

### 痛点 A：Op 的场景化泛滥

迁移前 `Op` 有 9 个变体：`StartTask`、`UserInput`、`Confirm`、`StructuredUserInput`、`RegisterDynamicTools`、`DynamicToolResult`、`Compact`、`Rollback`、`Cancel`。

其中 `Confirm`、`StructuredUserInput`、`DynamicToolResult` 本质上都是同一件事——**回复引擎的一个挂起请求**。但当时它们各自有独立的状态追踪代码，造成实现重复。

### 痛点 B：Thinking 是转瞬即逝的信号，不是磁带符号

迁移前 `Event::Thinking`、`Event::ReasoningDelta` 在流式阶段提供了思考过程，但 turn 结束后只保留最终文本写入 `Tape`。思考链未被持久化。

### 痛点 C：Message 是 API 适配层的产物，不是核心抽象

```rust
// 迁移前的 Message — 本质上是 OpenAI chat completion 格式的镜像
pub struct Message {
    pub role: MessageRole,
    pub content: String,                        // 文本偏向
    pub tool_name: Option<String>,              // 可选字段堆叠
    pub tool_payload: Option<serde_json::Value>, // 可选字段堆叠
    pub tool_calls: Option<Vec<ToolCall>>,       // 可选字段堆叠
}
```

一个扁平的 `String` content 加上一堆 `Option` 字段，无法原生表达多模态内容、结构化数据、或思考链。复杂数据被强行序列化为 JSON 字符串塞回 content——这是在扭曲图灵机的基本数据表达能力。

### 痛点 D：Event 的流式关注与语义状态混为一谈

迁移前 30+ 个 Event 变体中：
- `Thinking` / `ThinkingComplete` / `ReasoningDelta` 三个事件表达"模型在思考"
- `MessageDelta` / `MessageDeltaChunk` 两个事件表达"模型在输出文本"
- `ConfirmationRequired` / `StructuredUserInputRequested` 两个事件表达"引擎在等待输入"

流式传输的机制细节和语义状态转移混在同一层，导致每增加一种输出模态就要新增多个 Event 变体。

---

## 2. 设计哲学

> "Entities must not be multiplied beyond necessity." — Occam's Razor
>
> 但也不要把本质不同的实体强行合并成一个。

三条指导原则：

1. **内容与动作分离**：磁带上的符号（文本、思考、附件）和读写头的指令（工具调用、工具结果）不是同一个范畴。前者是名词，后者是动词。不要把它们塞进同一个 enum。

2. **收敛控制流，保留类型区分**：所有"等待外部输入"应统一为 Yield/Resume 对称结构。但流式输出的不同模态（文本流 vs 思考流 vs 工具参数流）在客户端有本质不同的渲染逻辑，应保留类型区分。

3. **磁带 ≠ LLM 输入**：磁带是完整的执行记录（包含 thinking、完整 tool result 等）。发给 LLM 的上下文是磁带的一个投影——可能需要截断、过滤、或按 provider 格式转换。不追求"内外同构"，而是在边界处做显式的 `project_for_llm()` 转换。

---

## 3. 内部磁带层：两层内容模型

### 3.1 ContentPart — 磁带上的合法符号

```rust
/// 磁带上的内容符号 — 图灵机字母表的基本单元。
/// 这些是"名词"：描述信息的载体。
pub enum ContentPart {
    /// 标准文本
    Text(String),

    /// 思考链 / 推理过程，持久化存储在磁带上。
    /// LLM 可以在后续 turn 中回溯自己的推理。
    Thinking(String),

    /// 多模态附件（图片、文件、音频等）
    Attachment {
        hash: String,
        mime_type: String,
        metadata: serde_json::Value,
    },

    /// 结构化数据的原生表达。
    /// 不再退化为 JSON 字符串塞进 Text。
    Structured(serde_json::Value),
}
```

### 3.2 ToolRequest / ToolResponse — 读写头的动作

```rust
/// 读写头发出的工具调用指令 — 这是"动词"。
pub struct ToolRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具执行的返回 — 动词的结果。
/// 结果本身是内容的组合，因此用 Vec<ContentPart> 表达。
pub struct ToolResponse {
    pub id: String,
    pub content: Vec<ContentPart>,
}
```

### 3.3 Message — 磁带上的完整记录

```rust
pub enum Message {
    /// 用户输入（可以包含文本、附件、结构化数据的任意组合）
    User {
        parts: Vec<ContentPart>,
    },

    /// 助手输出（内容 + 可选的工具调用请求）
    Assistant {
        parts: Vec<ContentPart>,
        tool_requests: Vec<ToolRequest>,
    },

    /// 工具执行结果
    Tool {
        responses: Vec<ToolResponse>,
    },

    /// 系统指令（system prompt、context injection 等）
    System {
        parts: Vec<ContentPart>,
    },
}
```

**为什么这样分层？**

- `ContentPart` 和 `ToolRequest` 的生命周期完全不同。文本和思考是被动的记录，工具调用是主动的指令。把它们放在同一个 enum 里（如之前的 `Block` 方案）会导致范畴混淆。
- `ToolResponse.content` 使用 `Vec<ContentPart>` 而不是 `String`，意味着工具可以返回富内容（截图、结构化数据），不再退化。
- `Assistant` 变体同时持有 `parts`（文本/思考内容）和 `tool_requests`（工具调用），反映了 LLM 响应的真实结构：一次响应可以同时包含文本输出和工具调用。

---

## 4. LLM 投影边界：`project_for_llm()`

磁带记录的是完整的执行历史。但不同 LLM provider 对上下文格式有不同要求：

- Anthropic 与 OpenAI-compatible 路径可保留 thinking/reasoning 元数据；Gemini 路径会丢弃 thinking（当前 wire format 不支持）
- 有些 provider 需要 tool_use / tool_result 的特定格式
- 长 tool result 可能需要截断以节省 token

因此，在磁带和 LLM 调用之间存在一个显式的投影层：

```rust
/// 将磁带消息投影为特定 LLM provider 能理解的格式。
/// 这是一个有损转换 — 磁带是 source of truth，投影是视图。
trait LlmProjection {
    fn project(&self, messages: &[Message], config: &ProjectionConfig) -> Vec<ProviderMessage>;
}
```

这个边界的存在意味着：
- Runtime 核心不需要关心 provider 的格式差异
- 磁带可以自由记录 Thinking 等内容，不用担心某些 provider 不支持
- 截断、过滤、格式转换都发生在明确的边界处，而不是散落在各处

---

## 5. 外部输入协议：Op

### 5.1 命名哲学

抛弃 `StartTask` 这个名字。从图灵机的视角看，不存在"启动任务"这个概念——只有"向磁带写入符号并启动读写头"。但从用户视角看，"开始一个新的对话 turn"和"在已有 turn 中追加输入"确实是不同的意图，它们携带的元数据也不同。

我们用 **Turn / Input / Resume / Interrupt** 四个动词来命名，每个都对应一个清晰的控制流语义：

### 5.2 Op 定义

```rust
pub enum Op {
    /// 开始一个新的推理 turn。
    /// 这是用户主动发起的对话轮次，携带完整的上下文元数据。
    Turn {
        parts: Vec<ContentPart>,
        /// 可选的工作区路由信息
        context: Option<TurnContext>,
    },

    /// 在已有 turn 中追加用户输入。
    /// 语义上等同于 steering message — 用户在引擎运行中插入新信息。
    Input {
        parts: Vec<ContentPart>,
    },

    /// 恢复一个挂起的 Yield 请求。
    /// 统一替代 Confirm、StructuredUserInput、DynamicToolResult。
    /// 引擎不关心这个回复来自确认弹窗、表单、还是外部工具——
    /// 它只知道：某个 request_id 的结果到了。
    Resume {
        request_id: String,
        content: Vec<ContentPart>,
    },

    /// 中断当前执行。
    Interrupt,

    /// 请求上下文压缩。
    Compact,

    /// 回滚最近的 N 个 turn。
    Rollback { turns: u32 },
}

/// Turn 的上下文元数据。
/// 不是 Op 的变体，而是 Turn 的附属数据。
pub struct TurnContext {
    pub workspace_id: Option<String>,
}
```

### 5.3 为什么不把 Turn 和 Input 合并？

它们的语义不同：

- `Turn` = "我要开始一个新的对话轮次"，引擎应该重置 turn 状态、记录 turn 边界、可能需要路由到特定 workspace。
- `Input` = "我要在当前 turn 中插入信息"（steering），引擎不应该重置状态，而是将输入缓冲或立即注入。

如果合并为一个 `Append`，引擎就需要通过 `context_override: Option<ContextMeta>` 的有无来猜测用户意图。这不是消灭复杂度，而是把类型系统能表达的语义推迟到运行时判断。

### 5.4 为什么 Resume 能统一三种回调？

看迁移前的 `TurnState`：

```rust
// 迁移前：三套并行的 pending 追踪
enum PendingTurnItem {
    Confirmation(PendingConfirmation),
    StructuredInput(PendingStructuredInputRequest),
    DynamicToolCall(PendingDynamicToolCall),
}
```

这三种 pending 的共同模式是：引擎发出请求 → 挂起 → 等待外部回复。区别仅在于请求的 payload 和回复的 payload 不同。

新设计中，引擎统一发出 `Event::Yield { request_id, kind, payload }`，客户端统一回复 `Op::Resume { request_id, content }`。其核心是统一 pending 键空间（下例为简化示意）：

```rust
// 新设计：一套统一的 pending 追踪
pub(crate) struct TurnState {
    pending: HashMap<String, PendingYield>,
    pending_order: Vec<String>,
    turn_activity: TurnActivityState,
    buffered_inband_submissions: VecDeque<Submission>,
}
```

三套互不相干的 pending 状态被统一为同一套请求 ID 机制，控制流显著收敛。

---

## 6. 外部输出协议：Event

### 6.1 设计原则

Event 服务于两个不同的关注点：

1. **流式传输**：客户端需要实时渲染文本、思考动画、工具执行进度。不同模态的流有不同的渲染逻辑（文本逐字显示 vs 思考区域动画 vs 工具进度条），因此保留类型区分。

2. **状态转移**：引擎的宏观状态变化（turn 开始/结束、挂起等待、错误）。这些应该尽量收敛。

### 6.2 Event 定义

```rust
pub enum Event {
    // ── Turn 生命周期 ──────────────────────────────────

    /// Turn 开始
    TurnStarted,

    /// Turn 正常结束
    TurnCompleted {
        summary: Option<String>,
    },

    // ── 流式输出（保留类型区分）────────────────────────

    /// 文本内容的增量流
    TextDelta {
        chunk: String,
        is_final: bool,
    },

    /// 思考过程的增量流
    /// 客户端渲染为折叠的思考区域，与文本流的 UI 处理完全不同
    ThinkingDelta {
        chunk: String,
        is_final: bool,
    },

    // ── 工具生命周期 ──────────────────────────────────

    /// 工具调用开始（供 UI 显示进度）
    ToolCallStarted {
        id: String,
        name: String,
    },

    /// 工具调用完成
    ToolCallCompleted {
        id: String,
        result_preview: Option<String>,
    },

    // ── 挂起等待（统一的 Yield）─────────────────────

    /// 引擎挂起，等待外部输入。
    /// 替代 ConfirmationRequired、StructuredUserInputRequested、
    /// 以及动态工具的隐式等待。
    ///
    /// `kind` 告诉客户端应该渲染什么 UI（确认弹窗、表单、等待指示器等），
    /// 但引擎不关心客户端如何渲染——它只等待对应的 Op::Resume。
    Yield {
        request_id: String,
        kind: YieldKind,
        payload: serde_json::Value,
    },

    // ── 系统事件 ─────────────────────────────────────

    /// 错误
    Error {
        message: String,
        recoverable: bool,
    },
}

/// Yield 的类型提示 — 告诉客户端"建议"渲染什么 UI。
/// 这是一个开放枚举，新增 kind 不需要修改引擎代码。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YieldKind {
    /// 需要用户确认（审批工具调用等）
    Confirmation,
    /// 需要用户填写结构化输入
    StructuredInput,
    /// 等待外部动态工具返回结果
    DynamicTool,
    /// 未来可扩展的其他类型...
    Custom(String),
}
```

### 6.3 为什么不把所有流式输出统一为 `AppendDelta(BlockDelta)`？

因为不同流的特征完全不同：

| 流类型 | 粒度 | 客户端行为 | 终止信号 |
|--------|------|-----------|---------|
| TextDelta | 逐 token | 追加到文本区域，光标闪烁 | `is_final: true` |
| ThinkingDelta | 逐 token | 追加到折叠的思考区域，不同的动画 | `is_final: true` |
| ToolCall 参数流 | JSON 增量 | 不直接渲染，内部缓冲 | ToolCallStarted/Completed |

一个统一的 `BlockDelta` 要么变成一个跟现在一样复杂的 enum（换了个名字而已），要么丢失类型信息让客户端用 `match` 猜测。保留顶层的类型区分更诚实。

---

## 7. 完整的数据流

```
用户操作                    引擎内部                      客户端渲染
────────                    ────────                      ────────

Op::Turn {parts}
    │
    ▼
  写入磁带: Message::User {parts}
    │
    ▼
  project_for_llm() ──→ LLM 调用
    │
    ▼
  流式响应 ──→ Event::ThinkingDelta (如有)
    │          Event::TextDelta
    │
    ▼
  解析 tool_calls?
    │
    ├─ 内置工具 ──→ Event::ToolCallStarted
    │               直接执行
    │               Event::ToolCallCompleted
    │               写入磁带: Message::Tool {responses}
    │               继续下一轮 LLM 调用
    │
    ├─ 需审批工具 ──→ Event::Yield {kind: Confirmation}
    │                  引擎挂起
    │                  ← Op::Resume {request_id, content}
    │                  写入磁带: Message::Tool {responses}
    │                  继续下一轮 LLM 调用
    │
    └─ 动态工具 ──→ Event::Yield {kind: DynamicTool}
                     引擎挂起
                     ← Op::Resume {request_id, content}
                     写入磁带: Message::Tool {responses}
                     继续下一轮 LLM 调用
    │
    ▼
  无更多 tool_calls
    │
    ▼
  写入磁带: Message::Assistant {parts, tool_requests: []}
    │
    ▼
  Event::TurnCompleted
```

---

## 8. 迁移策略

这不是一次 big-bang 重写。分三个阶段：

### Phase 1：内部磁带升级

- 将 `Message` 从扁平 `String` + `Option` 字段迁移到 `ContentPart` + `ToolRequest` 结构
- 在 `Tape` 层实现 `project_for_llm()` 边界
- 现有 `Op` 和 `Event` 暂不改动，在 runtime 内部做新旧 Message 的转换

### Phase 2：Op 收敛

- 引入 `Op::Turn` 替代 `Op::StartTask`
- 引入 `Op::Resume` 统一替代 `Confirm` / `StructuredUserInput` / `DynamicToolResult`
- 重构 `TurnState` 为统一的 `pending_yields` 模型
- 旧 Op 标记为 deprecated，客户端逐步迁移

### Phase 3：Event 收敛

- 引入 `Event::Yield` 统一替代 `ConfirmationRequired` / `StructuredUserInputRequested`
- 合并 `Thinking` + `ThinkingComplete` + `ReasoningDelta` 为 `ThinkingDelta`
- 合并 `MessageDelta` + `MessageDeltaChunk` 为 `TextDelta`
- 移除冗余的 Event 变体

### 当前落地状态（2026-02）

- Phase 1：已完成（`ContentPart` / `ToolRequest` / `ToolResponse` 与投影边界已落地）
- Phase 2：已完成（`Op::Turn` / `Op::Input` / `Op::Resume` / `Op::Interrupt` 已替代旧控制流）
- Phase 3：协议主线已完成（`Event::Yield` / `ThinkingDelta` / `TextDelta` 已是主事件）
- 兼容层：客户端和类型生成中仍保留少量历史兼容字段，用于渐进迁移

---

## 9. 设计收益

1. **TurnState 复杂度显著收敛**：挂起交互统一为同一 pending map（按 `request_id` 跟踪），减少并行状态机分支。

2. **Provider 中立**：`ContentPart` / `ToolRequest` 是 Alan 自己的抽象，不是任何 LLM API 的镜像。Provider 适配发生在 `project_for_llm()` 边界，Runtime 全局解耦。

3. **Thinking 持久化**：思考链成为磁带上的合法符号，LLM 可以在后续 turn 中回溯自己的推理过程。

4. **多模态原生支持**：`ContentPart::Attachment` 和 `ContentPart::Structured` 让富内容不再退化为 JSON 字符串。

5. **可扩展的 Yield 机制**：新增一种"等待外部输入"的场景，只需要新增一个 `YieldKind` 变体和对应的 payload schema，不需要新增 Op 变体、Event 变体、或 TurnState 追踪代码。

6. **Op 语义清晰**：`Turn` / `Input` / `Resume` / `Interrupt` 四个动词，每个都有不可替代的控制流语义，没有重叠，没有歧义。
