# Alan

**Alan** is a Rust-native Agent Runtime built around the **AI Turing Machine** metaphor — a state machine where LLMs drive transitions while the runtime manages tape (context), tooling, and persistence.

> **⚠️ Project Status: Early Development**
>
> This project is actively being developed. APIs may change without notice.
>
> Governance model note: policy/sandbox sections in this README reflect the accepted V2 breaking design and may be in migration until implementation is complete.

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
>
> 📚 **[Docs Index →](docs/README.md)**

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
| `alan-llm`      | Pluggable LLM adapters — OpenAI, OpenAI-compatible, Gemini, Anthropic-compatible (+ OpenRouter via adapter) |
| `alan-runtime`  | Core engine — session, tape, agent loop, tool registry, skills      |
| `alan-tools`    | Builtin tool implementations (`read_file`, `bash`, `grep`, etc.)    |
| `alan`          | Unified CLI & daemon — workspace lifecycle, HTTP/WS API, ask, chat  |

---

## Features

- **Multi-Provider LLM**: OpenAI, OpenAI-compatible, Gemini (Vertex AI), Anthropic-compatible
- **Streaming Responses**: Real-time token streaming with tool call support
- **Layered Tool Profiles**:
  - Core (default): `read_file`, `write_file`, `edit_file`, `bash`
  - Read-only exploration: `read_file`, `grep`, `glob`, `list_dir`
  - All built-ins: core + exploration tools (7 total)
- **Skill System**: Markdown-based capabilities via `$skill-name` triggers
- **Session Persistence**: Rollout recording with pause/resume/replay
- **Policy Over Sandbox**: Policy decides (`allow/deny/escalate`), sandbox enforces execution boundaries (current backend: workspace path guard with protected subpaths and only plain shell commands with statically addressable paths; shell control flow is rejected, common wrapper forms such as `env`/`command`/`builtin`/`exec`/`time`/`nice`/`nohup`/`timeout`/`stdbuf`/`setsid` are rejected, process path references under protected subpaths are blocked, glob patterns are rejected, direct nested shell/code evaluators are disabled, direct opaque command dispatchers such as `xargs`/`find -exec` are rejected, and a curated set of common direct script interpreters such as `python file.py`/`bash script.sh`/`awk -f script.awk` are rejected; the backend checks explicit path-like argv references and redirection targets but does not infer utility-specific operand roles for arbitrary bare tokens, and arbitrary program-internal writes or dispatch such as `git init`/`git add`/`git config --local`, `find -delete`, build/task runners, or utility-specific script/DSL modes like `sed -f` are not inspected by this backend and still need policy or a stronger OS sandbox; OS sandboxing is still in migration)
- **Policy Profiles**: Builtin `autonomous`/`conservative` presets, overridable via `.alan/policy.yaml`
- **Steering-First Execution**: In-turn `input` can interrupt tool batches and reprioritize the next step
- **WebSocket + HTTP API**: Real-time bidirectional communication
- **Context Compaction**: Automatic summarization when context grows large
- **One-Shot Ask**: `alan ask` for non-interactive queries with text/json/quiet output modes
- **Thinking Support**: Optional reasoning/thinking display with configurable token budget
- **Session Rollback**: Undo last N turns within a session

---

## Thinking / Reasoning Support

Alan exposes a unified `thinking_budget_tokens` switch in runtime config. Current provider behavior:

- **Anthropic-compatible**: native thinking blocks, thinking signature, and redacted thinking blocks; requires `budget_tokens >= 1024`
- **OpenAI**: Responses API-first with chat/completions fallback when needed
- **OpenAI-compatible (including OpenRouter-style endpoints)**: chat/completions-compatible path with reasoning field support (for example `reasoning_content` and reasoning metadata)
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

Create `~/.config/alan/config.toml`:

```toml
# LLM Provider: openai | openai_compatible | gemini | anthropic_compatible
llm_provider = "openai"
openai_api_key = "sk-..."
openai_base_url = "https://api.openai.com/v1"
openai_model = "gpt-4o"

# Legacy compatible path
# llm_provider = "openai_compatible"
# openai_compat_api_key = "sk-..."
# openai_compat_base_url = "https://api.openai.com/v1"
# openai_compat_model = "gpt-4o"

# Or Gemini (Vertex AI)
# llm_provider = "gemini"
# gemini_project_id = "your-project"
# gemini_location = "us-central1"       # default
# gemini_model = "gemini-2.0-flash"     # default

# Or Anthropic-compatible
# llm_provider = "anthropic_compatible"
# anthropic_compat_api_key = "sk-ant-..."
# anthropic_compat_base_url = "https://api.anthropic.com/v1"
# anthropic_compat_model = "claude-3-5-sonnet-latest"

# Optional compaction budgeting override
# context_window_tokens = 128000
# compaction_trigger_ratio = 0.8

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
# ask defaults to autonomous governance profile

# Workspace management
alan workspace list
alan workspace add ./my-project --name myproj
alan workspace remove myproj
alan workspace info myproj
```

### API Usage

```bash
# Create a session
# streaming_mode accepts auto | on | off
curl -X POST http://localhost:8090/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_dir": "/path/to/workspace",
    "governance": {"profile": "autonomous", "policy_path": ".alan/policy.yaml"},
    "streaming_mode": "on"
  }'

# Create a conservative session
curl -X POST http://localhost:8090/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"governance": {"profile": "conservative"}}'

# Create response sample fields
# {
#   "session_id": "...",
#   "websocket_url": "/api/v1/sessions/.../ws",
#   "events_url": "/api/v1/sessions/.../events",
#   "submit_url": "/api/v1/sessions/.../submit",
#   "governance": {...},
#   "streaming_mode": "on"
# }
# Note: 409 returned when the workspace already has an active runtime.

# Read session metadata + persisted messages
curl http://localhost:8090/api/v1/sessions/{id}/read

# Read persisted message history only
curl http://localhost:8090/api/v1/sessions/{id}/history

# Poll events from rollout gap-aware API
curl "http://localhost:8090/api/v1/sessions/{id}/events/read?after_event_id=e-123&limit=50"

# Response includes:
# {
#   "session_id": "...",
#   "gap": false,
#   "oldest_event_id": "e-100",
#   "latest_event_id": "e-123",
#   "events": [...]
# }

# Submit user input
curl -X POST http://localhost:8090/api/v1/sessions/{id}/submit \
  -H "Content-Type: application/json" \
  -d '{"op": {"type": "turn", "parts": [{"type": "text", "text": "Hello!"}]}}'

# Stream events (NDJSON)
curl -N http://localhost:8090/api/v1/sessions/{id}/events
```

### Policy Configuration (Optional)

Create `{workspace}/.alan/policy.yaml` to override builtin policy profile rules:

```yaml
rules:
  - id: deny-prod-delete
    tool: bash
    match_command: "kubectl delete"
    action: deny
    reason: protect production cluster

  - id: review-prod-deploy
    tool: bash
    match_command: "deploy --prod"
    action: escalate
    reason: explicit production boundary

default_action: allow
```

See [`docs/policy_over_sandbox.md`](docs/policy_over_sandbox.md) for details.

---

## Contributing

If you want to contribute, start with:

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- [SECURITY.md](SECURITY.md)
- [SUPPORT.md](SUPPORT.md)

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
