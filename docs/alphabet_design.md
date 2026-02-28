# Alphabet Design: Layered Symbol Set

In Alan's AI Turing Machine architecture, the **Alphabet** defines the symbols that can be read from and written to Tape, plus the I/O signals used to interact with the external world.

This document specifies the redesign of `Message` (internal tape symbol), `Op` (external input command), and `Event` (external output signal).

Core principle: **separate content from actions, converge control flow, and preserve meaningful type distinctions**.

---

## Document Position

This is a migration RFC + design rationale document with two parts:

- **Historical context**: why pre-migration complexity emerged
- **Target model**: the unified abstraction for future evolution

If you only care about current protocol truth, follow code:

- `crates/protocol/src/op.rs`
- `crates/protocol/src/event.rs`
- `crates/runtime/src/tape.rs`

---

## 1. Pre-Migration State (Historical Context)

### Pain A: Scenario-driven Op sprawl

Before migration, `Op` had many variants: `StartTask`, `UserInput`, `Confirm`, `StructuredUserInput`, `RegisterDynamicTools`, `DynamicToolResult`, `Compact`, `Rollback`, `Cancel`.

`Confirm`, `StructuredUserInput`, and `DynamicToolResult` were the same control pattern: **respond to one pending engine request**. Separate state tracking duplicated implementation.

### Pain B: Thinking was transient signal, not tape symbol

Pre-migration `Event::Thinking` and `Event::ReasoningDelta` streamed reasoning, but only final text persisted to `Tape`. Reasoning traces were not durably represented.

### Pain C: Message mirrored API shape instead of core abstraction

```rust
// Pre-migration Message (effectively mirroring OpenAI chat completion shape)
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_payload: Option<serde_json::Value>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

Flat `String` content plus layered `Option` fields could not natively express multimodal content, structured data, or reasoning chains. Rich data got serialized into text blobs.

### Pain D: Streaming concerns mixed with semantic state transitions

In 30+ event variants:

- multiple variants represented "model is thinking"
- multiple variants represented "model is streaming text"
- multiple variants represented "engine is waiting for input"

Transport details and semantic state transitions were blended, increasing protocol surface area.

---

## 2. Design Philosophy

> "Entities must not be multiplied beyond necessity." (Occam's Razor)
>
> But distinct entities must not be merged when their semantics differ.

Guidelines:

1. **Separate content from actions**
   Tape symbols (text/thinking/attachments) and head actions (tool call/result) are different categories.
2. **Converge control flow, preserve type distinctions**
   Waiting-for-input semantics unify under Yield/Resume, while streaming modalities keep distinct types for proper client rendering.
3. **Tape is not LLM input**
   Tape is full execution record; provider context is a projection of tape (`project_for_llm()`).

---

## 3. Internal Tape Layer: Two-tier Content Model

### 3.1 ContentPart: legal symbols on tape

```rust
/// Symbol unit on tape.
pub enum ContentPart {
    Text(String),

    /// Reasoning chain, persisted on tape.
    Thinking(String),

    /// Multimodal attachment.
    Attachment {
        hash: String,
        mime_type: String,
        metadata: serde_json::Value,
    },

    /// Native structured content.
    Structured(serde_json::Value),
}
```

### 3.2 ToolRequest / ToolResponse: actions of the read/write head

```rust
/// Tool call intent (verb).
pub struct ToolRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool result payload (action result).
pub struct ToolResponse {
    pub id: String,
    pub content: Vec<ContentPart>,
}
```

### 3.3 Message: full tape record

```rust
pub enum Message {
    User {
        parts: Vec<ContentPart>,
    },
    Assistant {
        parts: Vec<ContentPart>,
        tool_requests: Vec<ToolRequest>,
    },
    Tool {
        responses: Vec<ToolResponse>,
    },
    System {
        parts: Vec<ContentPart>,
    },
}
```

Why this layering works:

1. `ContentPart` and `ToolRequest` have different lifecycles and semantics.
2. `ToolResponse.content` can carry rich non-text outputs.
3. Assistant outputs can naturally include both content and tool requests in one response.

---

## 4. LLM Projection Boundary: `project_for_llm()`

Tape records complete execution history. Providers require different wire formats:

- some preserve reasoning metadata, some do not
- tool-call payload formats differ by provider
- large tool results may require truncation

Therefore an explicit projection boundary is required:

```rust
/// Project tape messages to provider-specific messages.
/// This is intentionally lossy: tape is truth, projection is view.
trait LlmProjection {
    fn project(&self, messages: &[Message], config: &ProjectionConfig) -> Vec<ProviderMessage>;
}
```

Benefits:

1. Runtime core is provider-neutral.
2. Tape can persist richer content than any single provider supports.
3. Truncation/filtering/format adaptation is localized to one boundary.

---

## 5. External Input Protocol: Op

### 5.1 Naming philosophy

Use four clear verbs with distinct control-flow semantics:

- `Turn`
- `Input`
- `Resume`
- `Interrupt`

### 5.2 Op definition

```rust
pub enum Op {
    /// Start a new reasoning turn.
    Turn {
        parts: Vec<ContentPart>,
        context: Option<TurnContext>,
    },

    /// Append user input during an active turn.
    Input {
        parts: Vec<ContentPart>,
    },

    /// Resume one pending yield request.
    Resume {
        request_id: String,
        content: Vec<ContentPart>,
    },

    Interrupt,
    Compact,
    Rollback { turns: u32 },
}

pub struct TurnContext {
    pub workspace_id: Option<String>,
}
```

### 5.3 Why not merge Turn and Input?

Their semantics differ:

1. `Turn` establishes a new turn boundary and context.
2. `Input` injects guidance into active turn without resetting state.

Merging both into one append primitive would force runtime intent inference at execution time.

### 5.4 Why Resume unifies prior callback variants

Old model tracked multiple pending item types separately.
New model emits one `Yield { request_id, kind, payload }` and expects one `Resume { request_id, content }`.

Unified pending-key space removes parallel state machines and simplifies turn-state tracking.

---

## 6. External Output Protocol: Event

### 6.1 Design principle

Events serve two concerns:

1. **Streaming rendering**: different modalities need different client rendering behavior.
2. **State transitions**: turn lifecycle and suspension semantics should converge.

### 6.2 Event definition

```rust
pub enum Event {
    // Turn lifecycle
    TurnStarted,
    TurnCompleted { summary: Option<String> },

    // Streaming output
    TextDelta { chunk: String, is_final: bool },
    ThinkingDelta { chunk: String, is_final: bool },

    // Tool lifecycle
    ToolCallStarted { id: String, name: String },
    ToolCallCompleted { id: String, result_preview: Option<String> },

    // Unified waiting state
    Yield {
        request_id: String,
        kind: YieldKind,
        payload: serde_json::Value,
    },

    // System
    Error { message: String, recoverable: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YieldKind {
    Confirmation,
    StructuredInput,
    DynamicTool,
    Custom(String),
}
```

### 6.3 Why not unify all streams under one `AppendDelta`?

Because stream modalities have materially different behavior:

| Stream | Granularity | Client behavior | Terminal signal |
| --- | --- | --- | --- |
| `TextDelta` | token-level | append to visible response text | `is_final: true` |
| `ThinkingDelta` | token-level | append to collapsible reasoning view | `is_final: true` |
| Tool argument stream | JSON increments | buffered internal parsing | tool lifecycle events |

A single delta type would either hide critical semantics or reintroduce the same complexity in another shape.

---

## 7. End-to-End Data Flow

```text
User Action                  Engine Internals                     Client Rendering
-----------                  ----------------                     ----------------

Op::Turn {parts}
    |
    v
Write tape: Message::User {parts}
    |
    v
project_for_llm() -> LLM call
    |
    v
Streaming -> Event::ThinkingDelta (optional)
          -> Event::TextDelta
    |
    v
Parse tool calls?
    |
    +-- Builtin tool -> Event::ToolCallStarted
    |                 execute tool
    |                 Event::ToolCallCompleted
    |                 write tape: Message::Tool {responses}
    |                 continue next LLM round
    |
    +-- Approval-required tool -> Event::Yield {kind: Confirmation}
    |                          suspend
    |                          <- Op::Resume {request_id, content}
    |                          write tape: Message::Tool {responses}
    |                          continue next LLM round
    |
    +-- Dynamic tool -> Event::Yield {kind: DynamicTool}
                       suspend
                       <- Op::Resume {request_id, content}
                       write tape: Message::Tool {responses}
                       continue next LLM round
    |
    v
No more tool calls
    |
    v
Write tape: Message::Assistant {parts, tool_requests: []}
    |
    v
Event::TurnCompleted
```

---

## 8. Migration Strategy

No big-bang rewrite. Three phases:

### Phase 1: Internal tape upgrade

1. Migrate `Message` from flat string/options to `ContentPart + ToolRequest`.
2. Introduce `project_for_llm()` at Tape boundary.
3. Keep existing Op/Event external shapes initially via internal conversion.

### Phase 2: Op convergence

1. Introduce `Op::Turn` replacing `Op::StartTask`.
2. Introduce `Op::Resume` replacing `Confirm` / `StructuredUserInput` / `DynamicToolResult`.
3. Refactor `TurnState` to unified pending-yield model.
4. Mark old Ops deprecated and migrate clients incrementally.

### Phase 3: Event convergence

1. Introduce `Event::Yield` replacing multiple waiting variants.
2. Merge reasoning stream variants into `ThinkingDelta`.
3. Merge message stream variants into `TextDelta`.
4. Remove redundant Event variants.

### Current rollout status (2026-02)

1. Phase 1: completed (`ContentPart` / `ToolRequest` / `ToolResponse` + projection boundary).
2. Phase 2: completed (`Op::Turn` / `Op::Input` / `Op::Resume` / `Op::Interrupt`).
3. Phase 3: protocol mainline completed (`Event::Yield` / `ThinkingDelta` / `TextDelta`).
4. Compatibility layer still keeps limited legacy fields for gradual migration.

---

## 9. Design Benefits

1. **TurnState complexity reduction**: one pending map keyed by `request_id`.
2. **Provider neutrality**: runtime abstractions are not mirrors of provider APIs.
3. **Reasoning persistence**: thinking becomes legal tape symbol for future turns.
4. **Native multimodal support**: structured and attachment content are first-class.
5. **Extensible yield mechanism**: add new wait types via `YieldKind` + payload schema.
6. **Clear Op semantics**: `Turn/Input/Resume/Interrupt` are non-overlapping control verbs.
