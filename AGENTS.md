# Alan - AI Agent Guide

> **⚠️ Project Status: Early Development**
>
> This project is actively being developed. APIs may change without notice.

---

## Core Concept: AI Turing Machine

Alan treats each agent as a **Turing machine** where the LLM is the transition function:

| TM Concept              | Alan Implementation                                                  |
| ----------------------- | -------------------------------------------------------------------- |
| **Tape**                | `Tape` — messages, context items, and conversation summary           |
| **Transition Function** | LLM generation — maps (state, input) → (action, new state)           |
| **State**               | `Session` — holds tape, tools, skills, and runtime config            |
| **Alphabet**            | Messages (user/assistant/tool) and tool calls                        |
| **Side Effects**        | Tool execution — the way the machine acts on the world               |
| **Halt**                | No more tool calls, final text response emitted                      |

`alan-runtime` is the generic machine; it knows nothing about hosting, deployment, or domain-specific behavior. All domain concerns live in outer crates.

---

## Technology Stack

| Aspect        | Technology                            |
| ------------- | ------------------------------------- |
| Language      | Rust (Edition 2024)                   |
| Build Tool    | Cargo + Just                          |
| Async Runtime | Tokio                                 |
| Web Framework | Axum                                  |
| Serialization | Serde (JSON, YAML)                    |
| Tracing       | tracing, tracing-subscriber           |
| HTTP Client   | reqwest                               |
| LLM Providers | Gemini, OpenAI, Anthropic, OpenRouter |
| License       | Apache License 2.0                    |

---

## Project Structure

```
Alan/
├── Cargo.toml                 # Workspace configuration
├── README.md                  # Project overview
├── AGENTS.md                  # This file
├── crates/
│   ├── protocol/              # Event/Op protocol (the "alphabet")
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── event.rs       # Event, EventEnvelope
│   │       └── op.rs          # Op, Submission
│   │
│   ├── llm/                   # LLM adapters (the "transition function")
│   │   └── src/
│   │       ├── lib.rs         # LlmProvider trait
│   │       ├── gemini.rs      # Google Gemini (Vertex AI)
│   │       ├── openai_compatible.rs
│   │       └── anthropic_compatible.rs
│   │
│   ├── core/                  # Core runtime (the "machine")
│   │   ├── prompts/           # Embedded prompt templates
│   │   │   ├── runtime_base.md
│   │   │   ├── system.md
│   │   │   └── persona/       # Workspace persona templates
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs      # Generic configuration
│   │       ├── tape.rs        # Tape + message/context types
│   │       ├── session.rs     # Session lifecycle + persistence
│   │       ├── approval.rs    # Tool approval + pending interaction types
│   │       ├── rollout.rs     # Rollout recording
│   │       ├── llm.rs         # LLM client wrapper
│   │       ├── manager/       # Agent state data types
│   │       │   ├── mod.rs
│   │       │   └── state.rs   # AgentState, AgentStatus
│   │       ├── runtime/       # Agent loop + turn execution
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs  # spawn(), AgentRuntimeConfig
│   │       │   ├── agent_loop.rs
│   │       │   ├── turn_driver.rs
│   │       │   ├── turn_executor.rs
│   │       │   ├── tool_orchestrator.rs
│   │       │   └── tool_policy.rs
│   │       ├── tools/         # Tool trait + registry
│   │       │   ├── mod.rs
│   │       │   └── registry.rs
│   │       ├── skills/        # Skill system
│   │       │   ├── mod.rs
│   │       │   ├── types.rs
│   │       │   ├── loader.rs
│   │       │   ├── registry.rs
│   │       │   └── injector.rs
│   │       └── prompts/       # Prompt assembly
│   │           ├── mod.rs     # SYSTEM_PROMPT, COMPACT_PROMPT, MEMORY_PROMPT
│   │           ├── loader.rs
│   │           ├── assembler.rs
│   │           └── workspace.rs
│   │
│   ├── tools/                 # Builtin tool implementations (alan-tools)
│   │   └── src/
│   │       └── lib.rs         # 7 tools: read/write/edit file, bash, grep, glob, list_dir
│   │
│   └── agentd/                # Agent daemon (hosting layer)
│       └── src/
│           ├── main.rs
│           ├── routes.rs      # HTTP API routes
│           ├── state.rs       # Application state
│           ├── websocket.rs   # WebSocket handler
│           └── manager/       # Agent lifecycle orchestration
│               ├── mod.rs
│               ├── instance.rs  # AgentInstance
│               └── agent_manager.rs   # AgentManager, ManagerConfig
│
└── clients/
    ├── tui/                   # Terminal UI (Bun + TypeScript)
    └── electron/              # Desktop client (Electron)
```

### Crate Dependency Graph

```
alan-protocol (base — no internal deps)
    ↑
alan-llm (depends on alan-protocol)
    ↑
alan-runtime (depends on alan-protocol, alan-llm)
    ↑        ↑
alan-tools   alan-agentd (depends on alan-protocol, alan-runtime)
```

---

## Build and Test Commands

### Using Just (Recommended)

```bash
just             # List available commands
just test        # Run all tests
just check       # Format + lint + test
just fmt         # Format code
just lint        # Clippy lints
just serve       # Run the daemon
just build       # Release build
```

### Using Cargo

```bash
cargo build --release
cargo test --workspace
cargo test -p alan-runtime
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run --bin agentd
```

---

## Code Style Guidelines

### Rustfmt Configuration

See `rustfmt.toml`: Edition 2024, 100-char max width, 4-space indent, alphabetical imports.

### Clippy Configuration

See `clippy.toml`: Cognitive complexity ≤ 30, enum variant ≤ 300 bytes, too-many-args ≤ 7.

### Coding Conventions

1. **Naming**: Standard Rust — `snake_case`, `PascalCase`, `SCREAMING_SNAKE_CASE`
2. **Error Handling**: `anyhow` for apps, `thiserror` for libs, `?` for propagation
3. **Async**: `tokio` runtime, `#[async_trait]` for trait async methods
4. **Observability**: `tracing` for structured logging (never `println!`)
5. **Documentation**: `///` doc comments on all public APIs

---

## Testing Strategy

Tests use inline `#[cfg(test)]` modules within source files. The `alan-llm` crate provides a `MockLlmProvider` (feature-gated via `mock`).

```bash
cargo test --workspace            # All tests
cargo test -p alan-runtime        # Core only
cargo test -p alan-tools          # Tool implementations
cargo test -p alan-agentd         # Daemon tests
```

---

## Configuration (Environment Variables)

```bash
# LLM Provider
LLM_PROVIDER=gemini                    # gemini | openai_compatible | anthropic_compatible

# Gemini
GEMINI_PROJECT_ID=your-project-id
GEMINI_LOCATION=us-central1
GEMINI_MODEL=gemini-2.0-flash

# OpenAI-compatible
OPENAI_COMPAT_API_KEY=sk-...
OPENAI_COMPAT_BASE_URL=https://api.openai.com/v1
OPENAI_COMPAT_MODEL=gpt-4o

# Anthropic-compatible
ANTHROPIC_COMPAT_API_KEY=sk-ant-...
ANTHROPIC_COMPAT_BASE_URL=https://api.anthropic.com/v1
ANTHROPIC_COMPAT_MODEL=claude-3-5-sonnet-latest

# Runtime
LLM_TIMEOUT_SECS=180
TOOL_TIMEOUT_SECS=30
MAX_TOOL_LOOPS=0                       # 0 = unlimited
TOOL_REPEAT_LIMIT=4

# Workspace
ALAN_WORKSPACE_DIR=~/.alan             # Override default workspace directory

# Server
BIND_ADDRESS=0.0.0.0:8090

# Memory
MEMORY_ENABLED=true
MEMORY_STRICT_WORKSPACE=true
```

---

## HTTP API

```bash
curl http://localhost:8090/health                              # Health check
curl -X POST http://localhost:8090/api/v1/sessions             # Create session
curl http://localhost:8090/api/v1/sessions                     # List sessions
curl http://localhost:8090/api/v1/sessions/{id}                # Get session
curl -X POST http://localhost:8090/api/v1/sessions/{id}/submit # Submit operation
curl -N http://localhost:8090/api/v1/sessions/{id}/events      # Stream events (NDJSON)
curl -X POST http://localhost:8090/api/v1/sessions/{id}/resume # Resume session
curl -X DELETE http://localhost:8090/api/v1/sessions/{id}      # Delete session
```

WebSocket: connect to `/api/v1/sessions/{id}/ws` for real-time bidirectional communication.

---

## Development Workflow

1. **Before committing**: `just check`
2. **Adding a new LLM provider**: implement `LlmProvider` in `crates/llm/src/`
3. **Adding new tools**: implement `Tool` trait in `crates/tools/src/`, register in `crates/runtime/src/tools/registry.rs`
4. **Adding skills**: create `SKILL.md` in `.alan/skills/` or `~/.config/alan/skills/`

---

## References

- **README.md**: Project philosophy and vision
- **docs/skill_and_tool_design.md**: Skill & Tool system design (Chinese)

### Inspirations

- [Claude Code](https://claude.ai) — human-style reasoning and collaboration
- [Codex](https://openai.com/blog/openai-codex) — intelligence expressed through code
- [pi-mono](https://github.com/badlogic/pi-mono/) — minimal agent runtime design
- **Turing Machine** — computation as state transitions on a tape

---

*Last updated: 2026-02-24*
*Project: Alan v0.1.0 (early development)*
