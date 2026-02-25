# Alan Architecture — The AI Turing Machine

## Philosophy

Alan models AI agents as **Turing machines**: a stateless program executes on a stateful tape, producing observable side effects. This simple metaphor gives us clean separation between *what the agent can do* (program), *who the agent is* (workspace), and *what it's doing right now* (session).

---

## Three-Layer Abstraction

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  AgentConfig                                      │  │
│  │  Stateless Program — "how to think"               │  │
│  │                                                   │  │
│  │  • LLM provider (Gemini, OpenAI, Anthropic)       │  │
│  │  • Model & parameters (temperature, tokens)       │  │
│  │  • Tool set (read, write, bash, grep, ...)        │  │
│  │  • Policies (approval, sandbox mode)              │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │ mounts into                          │
│  ┌───────────────▼───────────────────────────────────┐  │
│  │  Workspace                                        │  │
│  │  Persistent Context — "who I am"                  │  │
│  │                                                   │  │
│  │  • Identity (workspace_id)                        │  │
│  │  • Persona (SOUL.md, ROLE.md)                     │  │
│  │  • Memory (long-term knowledge)                   │  │
│  │  • Skills (markdown-based capabilities)           │  │
│  │  • Session archive (conversation history)         │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │ runs                                  │
│  ┌───────────────▼───────────────────────────────────┐  │
│  │  Session                                          │  │
│  │  Bounded Execution — "what I'm doing now"         │  │
│  │                                                   │  │
│  │  • Tape (messages + context)                      │  │
│  │  • LLM turns (input → generation → tool calls)   │  │
│  │  • Rollout (durable event log)                    │  │
│  │  • Limited by context window                      │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│                   AI Turing Machine                      │
└─────────────────────────────────────────────────────────┘
```

### Agent — The Program

An **Agent** is a stateless, reusable definition of *capabilities*. Like a CPU or a compiled program, it defines *how* to process information but holds no memory or identity of its own.

```rust
pub struct AgentConfig {
    pub core_config: Config,        // LLM engine: provider, model, timeouts
    pub runtime_config: RuntimeConfig, // behavior: approval policy, token limits
}
```

**Key properties:**
- **Stateless** — the same `AgentConfig` can power multiple Workspaces
- **Swappable** — changing the LLM provider is like swapping a CPU
- **Defines capability, not identity**

### Workspace — The Machine

A **Workspace** is the persistent, stateful context in which an Agent operates. It gives the agent its identity, memory, and working environment — like an operating system running on hardware.

```rust
pub struct WorkspaceRuntimeConfig {
    pub agent_config: AgentConfig,           // mounted program
    pub workspace_id: String,                // identity
    pub workspace_dir: Option<PathBuf>,      // persistent storage
    pub resume_rollout_path: Option<PathBuf>, // session restore point
}
```

**Workspace directory layout:**

```
{workspace_id}/
├── state.json          # workspace state (status, config, current session)
├── context/
│   └── skills/         # markdown-based capabilities
├── memory/
│   └── MEMORY.md       # long-term knowledge
└── sessions/
    ├── rollout-001.jsonl  # archived session
    └── rollout-002.jsonl  # current session
```

**Key properties:**
- **Persistent** — survives restarts, maintains identity across sessions
- **Self-contained** — all state lives in the workspace directory
- **Composable** — different Agents can be mounted into the same Workspace

### Session — The Computation

A **Session** is a single, bounded execution of an Agent within a Workspace. It represents one conversation or task, limited by the LLM's context window.

**Key properties:**
- **Bounded** — constrained by the context window; when full, start a new session
- **Archivable** — completed sessions are saved as rollouts for replay or forking
- **One active session per workspace** at any time; others are paused or archived

---

## Turing Machine Mapping

| TM Concept              | Alan Implementation                                          |
| ----------------------- | ------------------------------------------------------------ |
| **Program**             | `AgentConfig` — LLM + tools + policies                       |
| **Tape**                | `Tape` — messages, context items, conversation summary       |
| **Head**                | Current turn — reads tape, produces output                   |
| **Transition Function** | LLM generation — maps (state, input) → (action, new state)   |
| **State**               | `Session` — holds tape, tools, skills, and runtime config    |
| **Machine**             | `Workspace` — persistent identity + memory + session archive |
| **Alphabet**            | Messages (user/assistant/tool) and tool calls                |
| **Halt**                | No more tool calls, final text response emitted              |

---

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        Clients                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │   TUI    │  │ Electron │  │   API    │              │
│  │  (Bun)   │  │   (TS)   │  │ (HTTP/WS)│              │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘              │
└───────┼─────────────┼─────────────┼─────────────────────┘
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
        │             │             │ each run
        └─────────────┴─────────────┘
                      │
              ┌───────▼───────┐
              │  alan-runtime │  ← Agent runtime (transition function + tape)
              └───────┬───────┘
                      │
        ┌─────────────┼──────────────────┐
        │             │            │     │
   ┌────▼────┐  ┌─────▼─────┐ ┌───▼──┐ ┌▼────────┐
   │  alan   │  │   alan-   │ │alan  │ │  Tools  │
   │  -llm   │  │ protocol  │ │-tools│ │ (trait) │
   └─────────┘  └───────────┘ └──────┘ └─────────┘
```

### Crate Responsibilities

| Crate           | Role                                                             |
| --------------- | ---------------------------------------------------------------- |
| `alan-protocol` | Wire format — Events (output) and Operations (input)             |
| `alan-llm`      | Pluggable LLM adapters — Gemini, OpenAI, Anthropic, OpenRouter   |
| `alan-runtime`  | Core engine — session, tape, agent loop, tool registry, skills   |
| `alan-tools`    | Builtin tool implementations (`read_file`, `bash`, `grep`, etc.) |
| `alan-agentd`   | Hosting daemon — workspace lifecycle, HTTP/WS API, session mgmt  |

---

## Design Principles

1. **Stateless Agent, Stateful Workspace** — Clean separation between reusable computation logic and persistent identity/context.

2. **Checkpointed Reasoning** — Every thought, action, and observation is durably recorded in the session rollout.

3. **Generic Core** — `alan-runtime` is provider-agnostic, domain-agnostic, and hosting-agnostic. The same runtime powers different agents, workspaces, and deployment targets.

4. **Skills over Plugins** — Capabilities are defined as Markdown instructions that guide the agent's behavior, not as compiled code that extends the runtime.

5. **Bounded Sessions** — Context windows are finite. Instead of fighting this constraint, Alan embraces it: sessions are discrete, archivable units that can be summarized, forked, and resumed.
