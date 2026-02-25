# Alan

**Alan** is a Rust-native Agent Runtime built around the **AI Turing Machine** metaphor — a state machine where LLMs drive transitions while the runtime manages tape (context), tooling, and persistence.

> **⚠️ Project Status: Early Development**
>
> This project is actively being developed. APIs may change without notice.

---

## Core Concept: AI Turing Machine

Alan models AI agents as **Turing machines**: a stateless program executes on a stateful tape, producing observable side effects. This maps onto three clean abstractions:

| Abstraction   | Role                          | Analogy               |
| ------------- | ----------------------------- | --------------------- |
| **Agent**     | Stateless program             | CPU / compiled binary |
| **Workspace** | Persistent identity & context | OS + filesystem       |
| **Session**   | Bounded execution             | A single process run  |

```
  AgentConfig ──────► Workspace ──────► Session
  "how to think"     "who I am"       "what I'm doing now"
  (LLM + tools)      (persona +       (tape + turns +
                      memory +         rollout log)
                      skills)
```

> 📖 **[Full Architecture Documentation →](docs/architecture.md)**

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
              ┌───────▼────────┐
              │     agentd     │  ← Workspace lifecycle & hosting
              │WorkspaceManager│
              └───────┬────────┘
                      │ manages
        ┌─────────────┼─────────────┐
        │             │             │
   ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐
   │Workspace │ │Workspace │ │Workspace │  ← Persistent contexts
   │Instance 1│ │Instance 2│ │Instance N│
   └────┬─────┘ └────┬─────┘ └────┬─────┘
        │             │             │ each runs
        └─────────────┴─────────────┘
                      │
              ┌───────▼───────┐
              │  alan-runtime │  ← Agent runtime (transition fn + tape)
              └───────┬───────┘
                      │
        ┌─────────────┼──────────────────┐
        │             │            │     │
   ┌────▼────┐  ┌─────▼─────┐ ┌───▼──┐ ┌▼────────┐
   │  alan   │  │   alan-   │ │alan  │ │  Tools  │
   │  -llm   │  │ protocol  │ │-tools│ │ (trait) │
   └─────────┘  └───────────┘ └──────┘ └─────────┘
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

| Crate           | Role                                                             |
| --------------- | ---------------------------------------------------------------- |
| `alan-protocol` | Wire format — Events (output) and Operations (input)             |
| `alan-llm`      | Pluggable LLM adapters — Gemini, OpenAI, Anthropic, OpenRouter   |
| `alan-runtime`  | Core engine — session, tape, agent loop, tool registry, skills   |
| `alan-tools`    | Builtin tool implementations (`read_file`, `bash`, `grep`, etc.) |
| `alan-agentd`   | Hosting daemon — workspace lifecycle, HTTP/WS API, session mgmt  |

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
