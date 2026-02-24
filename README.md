# Alan

**Alan** is a Rust-native Agent Runtime built around the **AI Turing Machine** metaphor — a state machine where LLMs drive transitions while the runtime manages tape (context), tooling, and persistence.

> **⚠️ Project Status: Early Development**
>
> This project is actively being developed. APIs may change without notice.

---

## Core Concept: AI Turing Machine

Alan treats the agent as a **Turing machine** where the LLM is the transition function:

```
         ┌──────────────────────────────────────────────────────┐
         │                   AI Turing Machine                   │
         │                                                       │
  Input  │  ┌───────┐    ┌──────────┐    ┌──────────────────┐   │  Output
  ──────►│  │ Tape  │───►│   LLM    │───►│ Tool Execution   │──►│──────►
         │  │(context)   │(transition│   │  (side effects)  │   │  Events
         │  └───────┘    │ function) │   └──────────────────┘   │
         │       ▲       └──────────┘           │               │
         │       └──────────────────────────────┘               │
         │                  state transition                     │
         └──────────────────────────────────────────────────────┘
```

| TM Concept              | Alan Implementation                                                  |
| ----------------------- | -------------------------------------------------------------------- |
| **Tape**                | `Tape` — messages, context items, and conversation summary           |
| **Head**                | The current turn — reads tape, produces output                       |
| **Transition Function** | LLM generation — maps (state, input) → (action, new state)           |
| **State**               | `Session` — holds tape, tools, skills, and runtime config            |
| **Alphabet**            | Messages (user/assistant/tool) and tool calls                        |
| **Halt**                | No more tool calls, final text response emitted                      |

### Design Principles

1. **Generic Core** — `alan-runtime` is provider-agnostic, domain-agnostic, and hosting-agnostic
2. **Checkpointed Reasoning** — Every thought, action, and observation is durably recorded
3. **Separation of Concerns** — Core handles state transitions; hosting (agentd) handles lifecycle
4. **Skills over Plugins** — Capabilities are Markdown-based instructions, not compiled code

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                   │
│  │   TUI    │  │  Electron │  │   API    │                   │
│  │  (Bun)   │  │   (TS)   │  │ (HTTP/WS)│                   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                   │
└───────┼─────────────┼─────────────┼─────────────────────────┘
        │             │             │
        └─────────────┴─────────────┘
                      │
              ┌───────▼───────┐
              │    agentd     │  ← Agent lifecycle & hosting
              │ AgentManager  │
              │ AgentInstance  │
              └───────┬───────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
   ┌────▼────┐  ┌─────▼─────┐  ┌────▼────┐
   │  Agent  │  │  Agent    │  │  Agent  │  ← State machines
   │Runtime 1│  │ Runtime 2 │  │Runtime N│
   └────┬────┘  └─────┬─────┘  └────┬────┘
        │             │             │
        └─────────────┴─────────────┘
                      │
              ┌───────▼───────┐
              │  alan-runtime │  ← Transition function + tape
              └───────┬───────┘
                      │
        ┌─────────────┼──────────────────┐
        │             │             │    │
   ┌────▼────┐  ┌─────▼─────┐  ┌───▼──┐ ┌──▼──────┐
   │  alan   │  │  alan-   │  │alan  │ │  Tools  │
   │  -llm   │  │ protocol │  │-tools│ │(trait)  │
   └─────────┘  └───────────┘  └──────┘ └─────────┘
```

---

## Project Structure

```
Alan/
├── crates/
│   ├── protocol/     # Event/Op protocol definitions
│   ├── llm/          # LLM provider adapters (Gemini, OpenAI, Anthropic)
│   ├── core/         # Core runtime: tape, session, state transitions
│   ├── tools/        # Builtin tool implementations (alan-tools)
│   └── agentd/       # Agent daemon: lifecycle, HTTP/WS API
└── clients/
    ├── tui/          # Terminal UI (Bun + TypeScript)
    └── electron/     # Desktop client (Electron)
```

### Crates

| Crate        | Role in AI TM                                                      |
| ------------ | ------------------------------------------------------------------ |
| `alan-protocol` | Defines the **alphabet** — Events and Operations                |
| `alan-llm`   | Pluggable **transition functions** — LLM provider adapters         |
| `alan-runtime`  | The **machine** — tape, session, runtime loop, tool registry      |
| `alan-tools` | **Side effects** — 7 builtin tool implementations                  |
| `alan-agentd` | **Hosting** — agent lifecycle, multi-agent management, HTTP/WS API |

---

## Features

- **Multi-Provider LLM**: Gemini (Vertex AI), OpenAI, Anthropic-compatible, OpenRouter
- **Streaming Responses**: Real-time token streaming with tool call support
- **7 Core Tools**: `read_file`, `write_file`, `edit_file`, `bash`, `grep`, `glob`, `list_dir`
- **Skill System**: Markdown-based capabilities via `$skill-name` triggers
- **Session Persistence**: Rollout recording with pause/resume/replay
- **Sandbox Modes**: Read-only, workspace-write, or full access
- **Approval Policies**: Configurable approval for risky operations
- **WebSocket + HTTP API**: Real-time bidirectional communication
- **Context Compaction**: Automatic summarization when context grows large

---

## Quick Start

### Prerequisites

- Rust 1.85+ (2024 edition)

### Building

```bash
git clone <repo-url>
cd Alan
cargo build --release
cargo test --workspace
cargo run --bin agentd
```

### Configuration

Create a `.env` file:

```bash
# LLM Provider (gemini, openai_compatible, anthropic_compatible)
LLM_PROVIDER=gemini

# Gemini (Vertex AI)
GEMINI_PROJECT_ID=your-project
GEMINI_LOCATION=us-central1
GEMINI_MODEL=gemini-2.0-flash

# Server
BIND_ADDRESS=0.0.0.0:8090
```

### API Usage

```bash
# Create a session
curl -X POST http://localhost:8090/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"approval_policy": "on_request", "sandbox_mode": "workspace_write"}'

# Submit user input
curl -X POST http://localhost:8090/api/v1/sessions/{id}/submit \
  -H "Content-Type: application/json" \
  -d '{"op": {"type": "user_input", "content": "Hello!"}}'

# Stream events (NDJSON)
curl -N http://localhost:8090/api/v1/sessions/{id}/events
```

---

## Inspirations

- [Claude Code](https://claude.ai) — human-style reasoning and collaboration
- [Codex](https://openai.com/blog/openai-codex) — intelligence expressed through code
- [pi-mono](https://github.com/badlogic/pi-mono/) — minimal agent runtime design
- **Turing Machine** — computation as state transitions on a tape

---

## License

Apache License 2.0 — See [LICENSE](LICENSE) for details.
