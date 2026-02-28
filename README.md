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
3. **Separation of Concerns** — Core handles state transitions; the `alan` binary handles lifecycle & CLI
4. **Skills over Plugins** — Capabilities are Markdown-based instructions, not compiled code
5. **Human-in-the-End** — Humans own outcomes, not operations ([read more →](docs/human_in_the_end.md))

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │   TUI    │  │  alan    │  │   API    │                   │
│  │  (Bun)   │  │   ask    │  │ (HTTP/WS)│                   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                  │
└───────┼─────────────┼─────────────┼─────────────────────────┘
        │             │             │
        └─────────────┴─────────────┘
                      │
              ┌───────▼────────┐
              │      alan      │  ← Unified CLI & daemon
              │  daemon server │
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
│   ├── protocol/     # Event/Op protocol definitions + ContentPart
│   ├── llm/          # LLM provider adapters (Gemini, OpenAI, Anthropic)
│   ├── runtime/      # Core runtime: tape, session, agent loop, skills
│   ├── tools/        # Builtin tool implementations
│   └── alan/         # Unified CLI & daemon (ask, chat, workspace, daemon)
├── clients/
│   ├── tui/          # Terminal UI (Bun + TypeScript)
│   └── apple/        # Native Apple client (SwiftUI, macOS/iOS)
└── docs/             # Architecture, design philosophy, testing strategy
```

### Crates

| Crate           | Role                                                                |
| --------------- | ------------------------------------------------------------------- |
| `alan-protocol` | Wire format — Events (output), Operations (input), ContentPart      |
| `alan-llm`      | Pluggable LLM adapters — Gemini, OpenAI, Anthropic, OpenRouter      |
| `alan-runtime`  | Core engine — session, tape, agent loop, tool registry, skills      |
| `alan-tools`    | Builtin tool implementations (`read_file`, `bash`, `grep`, etc.)    |
| `alan`          | Unified CLI & daemon — workspace lifecycle, HTTP/WS API, ask, chat  |

---

## Features

- **Multi-Provider LLM**: Gemini (Vertex AI), OpenAI-compatible, Anthropic-compatible, OpenRouter
- **Streaming Responses**: Real-time token streaming with tool call support
- **Layered Tool Profiles**:
  - Core (default): `read_file`, `write_file`, `edit_file`, `bash`
  - Read-only exploration: `read_file`, `grep`, `glob`, `list_dir`
  - All built-ins: core + exploration tools (7 total)
- **Skill System**: Markdown-based capabilities via `$skill-name` triggers
- **Session Persistence**: Rollout recording with pause/resume/replay
- **Sandbox Modes**: Read-only, workspace-write, or full access
- **Approval Policies**: Configurable approval for risky operations
- **WebSocket + HTTP API**: Real-time bidirectional communication
- **Context Compaction**: Automatic summarization when context grows large
- **One-Shot Ask**: `alan ask` for non-interactive queries with text/json/quiet output modes
- **Thinking Support**: Optional reasoning/thinking display with configurable token budget
- **Session Rollback**: Undo last N turns within a session

---

## Thinking / Reasoning Support

Alan exposes a unified `thinking_budget_tokens` switch in runtime config. Current provider behavior:

- **Anthropic-compatible**: native thinking blocks, thinking signature, and redacted thinking blocks; requires `budget_tokens >= 1024`
- **OpenAI-compatible / OpenRouter**: supports `reasoning_effort` and parses provider reasoning fields (for example `reasoning_content` and reasoning metadata)
- **Gemini**: currently does not emit/consume thinking content in Alan's wire path

Notes:

- `thinking_budget_tokens = null` (default) means thinking budget is disabled.
- `alan ask --thinking` controls whether thinking deltas are shown in text-mode CLI output.

---

## Quick Start

### Prerequisites

- Rust 1.85+ (2024 edition)
- [just](https://github.com/casey/just) (task runner, optional but recommended)

### Building

```bash
git clone <repo-url>
cd Alan
cargo build --release

# Or use just
just build
```

### Configuration

Create `~/.alan/config.toml`:

```toml
# LLM Provider: gemini | openai_compatible | anthropic_compatible
llm_provider = "gemini"

# Gemini (Vertex AI)
gemini_project_id = "your-project"
gemini_location = "us-central1"       # default
gemini_model = "gemini-2.0-flash"     # default

# Or OpenAI-compatible
# llm_provider = "openai_compatible"
# openai_compat_api_key = "sk-..."
# openai_compat_base_url = "https://api.openai.com/v1"
# openai_compat_model = "gpt-4o"

# Or Anthropic-compatible
# llm_provider = "anthropic_compatible"
# anthropic_compat_api_key = "sk-ant-..."
# anthropic_compat_base_url = "https://api.anthropic.com/v1"
# anthropic_compat_model = "claude-3-5-sonnet-latest"

# Thinking / reasoning (optional)
# thinking_budget_tokens = 2048
```

You can also set `ALAN_CONFIG_PATH` to use a custom config file location.

### CLI Usage

```bash
# Initialize a workspace
alan init

# Start the daemon
alan daemon start              # background (default)
alan daemon start --foreground # foreground
alan daemon stop
alan daemon status

# Interactive chat (launches TUI)
alan chat

# One-shot question
alan ask "What does this project do?"
alan ask "Explain main.rs" --workspace ./my-project
alan ask "Summarize" --output json      # NDJSON for automation
alan ask "Summarize" --output quiet     # text only at end
alan ask "Think step by step" --thinking --timeout 60

# Workspace management
alan workspace list
alan workspace add ./my-project --name myproj
alan workspace remove myproj
alan workspace info myproj
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
  -d '{"op": {"type": "turn", "parts": [{"type": "text", "text": "Hello!"}]}}'

# Stream events (NDJSON)
curl -N http://localhost:8090/api/v1/sessions/{id}/events
```

---

## Development

```bash
just check          # format + lint + test
just fmt            # format code
just lint           # clippy
just test           # run all tests
just smoke          # mock smoke tests (no LLM needed)
just verify         # fmt + lint + test + smoke (run after code changes)
just verify-full    # verify + real LLM e2e test (needs ~/.alan config)
just coverage       # test coverage summary
just serve          # run the daemon in foreground
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
