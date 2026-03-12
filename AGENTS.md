# Alan - AI Agent Guide

> **⚠️ Project Status: Early Development**
>
> This project is actively being developed. APIs may change without notice.

---

## Core Concept: AI Turing Machine

Alan treats each agent as a **Turing machine** where the LLM is the transition function:

| TM Concept              | Alan Implementation                                        |
| ----------------------- | ---------------------------------------------------------- |
| **Tape**                | `Tape` — messages, context items, and conversation summary |
| **Transition Function** | LLM generation — maps (state, input) → (action, new state) |
| **State**               | `Session` — holds tape, tools, skills, and runtime config  |
| **Alphabet**            | Messages (user/assistant/tool) and tool calls              |
| **Side Effects**        | Tool execution — the way the machine acts on the world     |
| **Halt**                | No more tool calls, final text response emitted            |

`alan-runtime` is the generic machine; it knows nothing about hosting, deployment, or domain-specific behavior. All domain concerns live in outer crates.

### Three-Layer Abstraction

```
┌─────────────────────────────────────────────────────────────┐
│  AgentConfig                                                │
│  Stateless Program — "how to think"                         │
│  • LLM provider, model, parameters                          │
│  • Tool set, policies                                       │
├─────────────────────────────────────────────────────────────┤
│  Workspace                                                  │
│  Persistent Context — "who I am"                            │
│  • Identity, persona, memory, skills                        │
├─────────────────────────────────────────────────────────────┤
│  Session                                                    │
│  Bounded Execution — "what I'm doing now"                   │
│  • Tape (messages), rollout (event log)                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

| Aspect        | Technology                            |
| ------------- | ------------------------------------- |
| Language      | Rust (Edition 2024)                   |
| Build Tool    | Cargo + Just                          |
| Async Runtime | Tokio                                 |
| Web Framework | Axum                                  |
| Serialization | Serde (JSON, YAML, TOML)              |
| Tracing       | tracing, tracing-subscriber           |
| HTTP Client   | reqwest                               |
| LLM Providers | OpenAI Responses API, OpenAI Chat Completions API, OpenAI Chat Completions API-compatible, Google Gemini GenerateContent API, Anthropic Messages API (runtime); OpenRouter via adapter |
| License       | Apache License 2.0                    |

---

## Project Structure

```
Alan/
├── Cargo.toml                 # Workspace configuration
├── README.md                  # Project overview
├── AGENTS.md                  # This file
├── justfile                   # Development tasks
├── rustfmt.toml               # Code formatting config
├── clippy.toml                # Lint configuration
├── .tarpaulin.toml            # Code coverage config
├── crates/
│   ├── protocol/              # Event/Op protocol (the "alphabet")
│   │   └── src/
│   │       ├── lib.rs         # Re-exports
│   │       ├── event.rs       # Event, EventEnvelope (turn/text/thinking/tool/yield/error)
│   │       └── op.rs          # Op, Submission, GovernanceConfig, ToolCapability
│   │
│   ├── llm/                   # LLM adapters (the "transition function")
│   │   └── src/
│   │       ├── lib.rs         # LlmProvider trait, Message, ToolDefinition (+ MockLlmProvider feature)
│   │       ├── google_gemini_generate_content.rs  # Google Gemini GenerateContent API
│   │       ├── openai_responses.rs
│   │       ├── openai_chat_completions.rs
│   │       └── anthropic_messages.rs
│   │
│   ├── runtime/               # Core runtime (the "machine")
│   │   ├── prompts/           # Embedded prompt templates
│   │   │   ├── runtime_base.md
│   │   │   ├── system.md
│   │   │   └── persona/       # Workspace persona templates
│   │   │       ├── AGENTS.md
│   │   │       ├── BOOTSTRAP.md
│   │   │       ├── HEARTBEAT.md
│   │   │       ├── ROLE.md
│   │   │       ├── SOUL.md
│   │   │       ├── TOOLS.md
│   │   │       └── USER.md
│   │   ├── skills/            # Built-in system skills
│   │   │   ├── memory/SKILL.md
│   │   │   ├── plan/SKILL.md
│   │   │   └── workspace-manager/SKILL.md
│   │   └── src/
│   │       ├── lib.rs         # Public exports
│   │       ├── config.rs      # Config (TOML file-based + selected env overrides)
│   │       ├── tape.rs        # Tape (messages, context, compaction)
│   │       ├── session.rs     # Session lifecycle + persistence
│   │       ├── approval.rs    # Tool escalation cache + pending interaction types
│   │       ├── policy.rs      # Policy engine (policy over sandbox)
│   │       ├── rollout.rs     # JSONL persistence format
│   │       ├── llm.rs         # LlmClient wrapper
│   │       ├── retry.rs       # Retry logic with backoff
│   │       ├── manager/
│   │       │   ├── mod.rs
│   │       │   └── state.rs   # WorkspaceConfigState, WorkspaceInfo
│   │       ├── prompts/
│   │       │   ├── assembler.rs
│   │       │   ├── loader.rs
│   │       │   └── workspace.rs
│   │       ├── runtime/       # Agent loop + turn execution
│   │       │   ├── mod.rs     # RuntimeConfig
│   │       │   ├── engine.rs  # spawn(), RuntimeHandle
│   │       │   ├── agent_loop.rs
│   │       │   ├── turn_driver.rs
│   │       │   ├── turn_executor.rs
│   │       │   ├── turn_state.rs
│   │       │   ├── turn_support.rs
│   │       │   ├── tool_orchestrator.rs
│   │       │   ├── tool_policy.rs
│   │       │   ├── virtual_tools.rs
│   │       │   ├── loop_guard.rs
│   │       │   └── submission_handlers.rs
│   │       ├── skills/        # Skill system
│   │       │   ├── mod.rs
│   │       │   ├── types.rs
│   │       │   ├── loader.rs
│   │       │   ├── registry.rs
│   │       │   └── injector.rs
│   │       └── tools/         # Tool trait + registry
│   │           ├── mod.rs
│   │           ├── context.rs
│   │           ├── registry.rs
│   │           └── sandbox.rs
│   │
│   ├── tools/                 # Builtin tool implementations (alan-tools)
│   │   └── src/
│   │       └── lib.rs         # Tool profiles: core(4), read-only(4), all(7)
│   │
│   └── alan/                  # CLI & daemon (alan binary)
│       └── src/
│           ├── main.rs        # CLI entry point (clap)
│           ├── lib.rs         # Library exports
│           ├── cli/           # CLI commands
│           │   ├── mod.rs
│           │   ├── init.rs    # `alan init` command
│           │   ├── workspace.rs # `alan workspace` commands
│           │   ├── chat.rs    # `alan chat` command (launches TUI)
│           │   ├── ask.rs     # `alan ask` command
│           │   └── daemon.rs  # Daemon control commands
│           ├── daemon/        # HTTP/WebSocket server
│           │   ├── mod.rs
│           │   ├── server.rs  # Axum server setup
│           │   ├── routes.rs  # HTTP API routes
│           │   ├── state.rs   # AppState
│           │   ├── websocket.rs
│           │   ├── workspace_resolver.rs
│           │   ├── runtime_manager.rs
│           │   └── session_store.rs
│           └── registry.rs    # Workspace registry (CLI)
│
└── clients/
    ├── tui/                   # Terminal UI (Bun + TypeScript + Ink)
    └── apple/                 # Native Apple client (SwiftUI, macOS/iOS)
```

### Crate Dependency Graph

```
alan-protocol (base — no internal deps)
    ↑
alan-llm (depends on alan-protocol)
    ↑
alan-runtime (depends on alan-protocol, alan-llm)
    ↑        ↑
alan-tools   alan (depends on alan-protocol, alan-runtime)
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
just install     # Install to ~/.alan/bin
just uninstall   # Remove from ~/.alan/bin
just clean       # Clean build artifacts
just coverage    # Show coverage summary
just coverage-detail    # Detailed coverage
just coverage-html      # HTML coverage report
```

### Using Cargo

```bash
cargo build --release
cargo test --workspace
cargo test -p alan-runtime
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run --bin alan
```

---

## Code Style Guidelines

### Rustfmt Configuration

See `rustfmt.toml`: Edition 2024, 100-char max width, 4-space indent, alphabetical imports.

```toml
edition = "2024"
max_width = 100
tab_spaces = 4
hard_tabs = false
newline_style = "Unix"
reorder_imports = true
use_field_init_shorthand = true
```

### Clippy Configuration

See `clippy.toml`: Cognitive complexity ≤ 30, enum variant ≤ 300 bytes, too-many-args ≤ 7.

```toml
cognitive-complexity-threshold = 30
enum-variant-size-threshold = 300
too-many-arguments-threshold = 7
too-many-lines-threshold = 100
type-complexity-threshold = 250
```

### Coding Conventions

1. **Naming**: Standard Rust — `snake_case`, `PascalCase`, `SCREAMING_SNAKE_CASE`
2. **Error Handling**: `anyhow` for apps, `thiserror` for libs, `?` for propagation
3. **Async**: `tokio` runtime, `#[async_trait]` for trait async methods
4. **Observability**: `tracing` for structured logging (never `println!`)
5. **Documentation**: `///` doc comments on all public APIs
6. **Module structure**: Each module has `mod.rs` or is a file with submodules

---

## Testing Strategy

Tests include both inline `#[cfg(test)]` modules and integration tests (for example `crates/alan/tests/*`). The `alan-llm` crate provides a `MockLlmProvider` (feature-gated via `mock`).

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p alan-runtime
cargo test -p alan-tools
cargo test -p alan-protocol
cargo test -p alan-llm

# Run with mock feature
cargo test -p alan-llm --features mock
```

### Test Patterns

- Unit tests are in the same file as the code they test
- Use `tempfile::TempDir` for filesystem tests
- Use `MockLlmProvider` for testing LLM-dependent code
- All protocol types have serialization/deserialization tests

---

## Configuration

### Environment Variables (Directly Read by Runtime/CLI)

```bash
# Config file path override
ALAN_CONFIG_PATH=/absolute/path/to/config.toml

# Server
BIND_ADDRESS=0.0.0.0:8090

# CLI daemon endpoint override
ALAN_AGENTD_URL=http://127.0.0.1:8090

# Optional custom TUI bundle path for `alan chat`
ALAN_TUI_PATH=/absolute/path/to/alan-tui.js
```

LLM/provider/timeouts/memory/tool-loop settings are loaded from `~/.config/alan/config.toml` (or `ALAN_CONFIG_PATH`), not from per-key environment variables.

### Config File

Configuration can also be loaded from `~/.config/alan/config.toml`:

If you launch `alan chat` or `alan-tui` without a config file, the first-run wizard starts
with user-facing service presets such as OpenAI API Platform, OpenRouter, Kimi Coding,
DeepSeek, Google Gemini via Vertex AI, and Anthropic API. Raw API-family selection is kept
behind `Advanced / custom setup`, but the generated file still uses the canonical provider
surface shown below.

```toml
# openai_responses | openai_chat_completions | openai_chat_completions_compatible
# google_gemini_generate_content | anthropic_messages
llm_provider = "openai_responses"
openai_responses_api_key = "sk-..."
openai_responses_base_url = "https://api.openai.com/v1"
openai_responses_model = "gpt-5.4"

# OpenAI Chat Completions API
# llm_provider = "openai_chat_completions"
# openai_chat_completions_api_key = "sk-..."
# openai_chat_completions_base_url = "https://api.openai.com/v1"
# openai_chat_completions_model = "gpt-5.4"

# OpenAI Chat Completions API-compatible
# llm_provider = "openai_chat_completions_compatible"
# openai_chat_completions_compatible_api_key = "sk-..."
# openai_chat_completions_compatible_base_url = "https://api.openai.com/v1"
# openai_chat_completions_compatible_model = "qwen3.5-plus"

# Google Gemini GenerateContent API
# llm_provider = "google_gemini_generate_content"
# google_gemini_generate_content_project_id = "your-project"
# google_gemini_generate_content_location = "us-central1"
# google_gemini_generate_content_model = "gemini-2.0-flash"

# Anthropic Messages API
# llm_provider = "anthropic_messages"
# anthropic_messages_api_key = "sk-ant-..."
# anthropic_messages_base_url = "https://api.anthropic.com/v1"
# anthropic_messages_model = "claude-3-5-sonnet-latest"

llm_request_timeout_secs = 180
tool_timeout_secs = 30
max_tool_loops = 0
tool_repeat_limit = 4
# Optional explicit override; otherwise derived from the model catalog
context_window_tokens = 128000
compaction_trigger_ratio = 0.8
prompt_snapshot_enabled = false
prompt_snapshot_max_chars = 8000
# Optional provider reasoning/thinking budget
# thinking_budget_tokens = 2048

[memory]
enabled = true
strict_workspace = true
```

If you are upgrading from older provider names such as `openai`, `openai_compatible`,
`anthropic_compatible`, or `gemini`, Alan now exits with an explicit migration error
instead of silently falling back to defaults. Migrate existing files with:

```bash
alan migrate terminology --write
# Or target a specific workspace / config file:
alan migrate terminology --write --workspace /path/to/workspace
alan migrate terminology --write --config-path ~/.config/alan/config.toml
```

Model metadata resolves in this order:

1. Bundled catalog
2. `~/.alan/models.toml`
3. `{workspace}/.alan/models.toml`

Overlay catalogs currently extend `openai_chat_completions_compatible` models only. Official
`openai_responses` and `openai_chat_completions` models stay pinned to Alan's curated catalog.

---

## HTTP API

The daemon exposes REST and WebSocket endpoints:

```bash
# Health check
curl http://localhost:8090/health

# Create session
curl -X POST http://localhost:8090/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_dir": "/path/to/workspace",
    "governance": {"profile": "conservative", "policy_path": ".alan/policy.yaml"},
    "streaming_mode": "on"
  }'

# Create autonomous session (fewer runtime interruptions)
curl -X POST http://localhost:8090/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"governance": {"profile": "autonomous"}}'

# Create-session response includes:
# {
#   "session_id": "...",
#   "websocket_url": "/api/v1/sessions/.../ws",
#   "events_url": "/api/v1/sessions/.../events",
#   "submit_url": "/api/v1/sessions/.../submit",
#   "governance": {...},
#   "streaming_mode": "on"
# }
# Note: returns 409 when the workspace already has an active runtime.

# List sessions
curl http://localhost:8090/api/v1/sessions

# Get session
curl http://localhost:8090/api/v1/sessions/{id}

# Read session metadata + persisted message history
curl http://localhost:8090/api/v1/sessions/{id}/read

# Read persisted message history only
curl http://localhost:8090/api/v1/sessions/{id}/history

# Submit operation (start a new turn)
curl -X POST http://localhost:8090/api/v1/sessions/{id}/submit \
  -H "Content-Type: application/json" \
  -d '{"op": {"type": "turn", "parts": [{"type": "text", "text": "Hello!"}]}}'

# Stream events (NDJSON)
curl -N http://localhost:8090/api/v1/sessions/{id}/events

# Read buffered events (poll API)
curl "http://localhost:8090/api/v1/sessions/{id}/events/read?after_event_id=e-123&limit=50"
# Response includes:
# {
#   "session_id": "...",
#   "gap": false,
#   "oldest_event_id": "e-100",
#   "latest_event_id": "e-123",
#   "events": [...]
# }

# Resume stalled runtime channel (server-side recovery)
curl -X POST http://localhost:8090/api/v1/sessions/{id}/resume

# Fork session from latest rollout
curl -X POST http://localhost:8090/api/v1/sessions/{id}/fork

# Roll back in-memory turns (non-durable; does not survive restart)
curl -X POST http://localhost:8090/api/v1/sessions/{id}/rollback \
  -H "Content-Type: application/json" \
  -d '{"turns": 2}'
# Response includes:
# {
#   "submission_id": "...",
#   "accepted": true,
#   "durability": {"durable": false, "scope": "in_memory"},
#   "warning": "Rollback is in-memory only and will not survive runtime restart."
# }

# Trigger manual context compaction
curl -X POST http://localhost:8090/api/v1/sessions/{id}/compact

# Delete session
curl -X DELETE http://localhost:8090/api/v1/sessions/{id}
```

WebSocket: connect to `/api/v1/sessions/{id}/ws` for real-time bidirectional communication.

---

## Key Concepts

### Events (Output Protocol)

Events are emitted by the runtime to notify frontends of state changes:

- `TurnStarted` / `TurnCompleted` — Turn boundaries
- `ThinkingDelta` — Streaming reasoning content
- `TextDelta` — Streaming response content
- `ToolCallStarted` / `ToolCallCompleted` — Tool execution
- `Yield` — Engine is suspended and waiting for external input
- `Error` — Something went wrong

### Operations (Input Protocol)

Operations are submitted by users to control the agent:

- `Turn` — Start a new user turn
- `Input` — Append user input to an active turn
- `Resume` — Resume a pending `Yield` request
- `Interrupt` — Stop current execution
- `RegisterDynamicTools` — Add client-provided tools
- `Compact` — Trigger context compaction
- `Rollback` — Roll back N turns

### Tools

`alan-tools` provides layered built-in tool profiles:

- **Core (default)**: `read_file`, `write_file`, `edit_file`, `bash`
- **Read-only exploration**: `read_file`, `grep`, `glob`, `list_dir`
- **All built-ins**: core + exploration tools (7 total)

Tool details:

| Tool         | Capability | Description                            |
| ------------ | ---------- | -------------------------------------- |
| `read_file`  | Read       | Read file contents (with offset/limit) |
| `write_file` | Write      | Write content to file                  |
| `edit_file`  | Write      | Search/replace text in file            |
| `bash`       | Read/Write/Network (dynamic) | Execute shell commands     |
| `grep`       | Read       | Search file contents with regex        |
| `glob`       | Read       | Find files matching pattern            |
| `list_dir`   | Read       | List directory contents                |

Runtime virtual tools (not from `alan-tools`, injected by runtime):

- `request_confirmation` — pause and emit `Yield(confirmation)`
- `request_user_input` — pause and emit `Yield(structured_input)`
- `update_plan` — update in-memory plan state in current turn

### Tool Governance

Runtime applies tool decisions in two stages:

1. `PolicyEngine` returns `allow | escalate | deny`.
2. If execution proceeds, the current `workspace_path_guard` backend applies a best-effort execution guard for workspace paths and shell shape checks.

Session governance is configured via:

```json
{
  "governance": {
    "profile": "autonomous",
    "policy_path": ".alan/policy.yaml"
  }
}
```

Policy resolution order is:

1. `governance.policy_path`, if provided
2. `{workspace}/.alan/policy.yaml`, if present
3. builtin profile defaults

When a policy file is found, it replaces the builtin profile rule set for that session.

### Skills

Skills are Markdown-based capabilities with YAML frontmatter:

```markdown
---
name: skill-name
description: What this skill does
metadata:
  short-description: Brief description
  tags: ["tag1", "tag2"]
---

# Instructions

Step-by-step guidance for the agent...
```

Skills can be triggered:
1. Explicitly: `$skill-name` in user input
2. Implicitly: LLM selects based on description matching

Skill scopes:
- `[system]` — Built into the binary
- `[user]` — In `~/.alan/skills/`
- `[repo]` — In `{workspace}/.alan/skills/`

---

## Development Workflow

1. **Before committing**: `just check`
2. **Adding a new LLM provider**: Implement `LlmProvider` trait in `crates/llm/src/`
3. **Adding new tools**: Implement `Tool` trait in `crates/tools/src/`, register via `create_core_tools()`
4. **Adding skills**: Create `SKILL.md` in `crates/runtime/skills/` or workspace/user directories

---

## Installation

```bash
# Clone and build
git clone <repo-url>
cd Alan
./scripts/install.sh

# Add to PATH (fish)
set -gx PATH $HOME/.alan/bin $PATH

# Add to PATH (bash/zsh)
export PATH="$HOME/.alan/bin:$PATH"

# Run
alan  # or: alan chat
```

---

## References

- **README.md**: Project philosophy and vision
- **docs/architecture.md**: Full architecture documentation
- **docs/governance_current_contract.md**: Authoritative current governance contract
- **docs/policy_over_sandbox.md**: Policy-over-sandbox runtime model

### Inspirations

- [Claude Code](https://claude.ai) — human-style reasoning and collaboration
- [Codex](https://openai.com/blog/openai-codex) — intelligence expressed through code
- [pi-mono](https://github.com/badlogic/pi-mono/) — minimal agent runtime design
- **Turing Machine** — computation as state transitions on a tape

---

*Last updated: 2026-02-28*
*Project: Alan v0.1.0 (early development)*
