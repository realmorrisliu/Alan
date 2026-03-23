# Alan Architecture — The AI Turing Machine

> Status: this document tracks the current architecture plus the accepted V2
> governance direction.
>
> Current governance semantics are defined in
> [`governance_current_contract.md`](./governance_current_contract.md). When this
> document discusses stricter future sandboxing, treat that as target-state
> design rather than a statement about today's implementation.

## Philosophy

Alan models AI agents as **Turing machines**: LLM generation is the transition
function, the tape holds bounded conversational state, and tools are the side
effects. That computation model is intentionally separate from Alan's hosting
model, which distinguishes on-disk agent definitions, persistent workspaces,
running agent instances, and bounded sessions.

Companion execution contracts:

- [`spec/kernel_contract.md`](./spec/kernel_contract.md)
- [`spec/execution_model.md`](./spec/execution_model.md)
- [`spec/memory_architecture.md`](./spec/memory_architecture.md)
- [`spec/compaction_contract.md`](./spec/compaction_contract.md)
- [`spec/governance_boundaries.md`](./spec/governance_boundaries.md)
- [`spec/app_server_protocol.md`](./spec/app_server_protocol.md)
- [`spec/scheduler_contract.md`](./spec/scheduler_contract.md)
- [`spec/interaction_inbox_contract.md`](./spec/interaction_inbox_contract.md)
- [`spec/durable_run_contract.md`](./spec/durable_run_contract.md)
- [`spec/extension_contract.md`](./spec/extension_contract.md)
- [`spec/capability_router.md`](./spec/capability_router.md)
- [`spec/harness_bridge.md`](./spec/harness_bridge.md)
- [`autonomy_layered_design.md`](./autonomy_layered_design.md)

---

## Hosting + Computation Model

```
┌─────────────────────────────────────────────────────────┐
│  HostConfig                                             │
│  Machine-local host settings (`~/.alan/host.toml`)      │
├─────────────────────────────────────────────────────────┤
│  AgentRoot                                              │
│  On-disk definition: agent.toml, persona, skills, policy│
├─────────────────────────────────────────────────────────┤
│  Workspace                                              │
│  Persistent identity, memory, sessions, workspace state │
├─────────────────────────────────────────────────────────┤
│  AgentInstance                                          │
│  Running process bound to a resolved agent definition   │
├─────────────────────────────────────────────────────────┤
│  Session                                                │
│  Bounded tape + rollout for the current task            │
└─────────────────────────────────────────────────────────┘
```

`SpawnSpec` is the explicit child-agent launch contract that will connect
agent-instance supervision with future multi-agent execution. Runtime-internal
types such as `AgentConfig` still exist, but they are derived from resolved
agent roots rather than serving as Alan's primary user-facing hosting model.

### AgentRoot — The On-Disk Definition

An **AgentRoot** is the filesystem form of an agent definition. Alan resolves one
effective agent by overlaying multiple roots.

```text
~/.alan/agent/                   # global base agent root
~/.alan/agents/<name>/           # global named agent root
<workspace>/.alan/agent/         # workspace base agent root
<workspace>/.alan/agents/<name>/ # workspace named agent root
```

Each root may contain:

- `agent.toml`
- `persona/`
- `skills/`
- `policy.yaml`

Overlay order is:

- Default workspace agent: `~/.alan/agent -> <workspace>/.alan/agent`
- Named agent: `~/.alan/agent -> <workspace>/.alan/agent -> ~/.alan/agents/<name> -> <workspace>/.alan/agents/<name>`

This overlay chain defines an agent. It is not runtime process ancestry.

### Capability Packages In The Definition Layer

Each resolved `AgentRoot` contributes its `skills/` directory as a capability
package source. Alan combines those root-backed sources with built-in
first-party packages into one `ResolvedCapabilityView`, which is then consumed
by runtime instead of the older mixed `repo/user/system` skill-loading paths.

A standards-compatible skill directory with `SKILL.md` and optional supporting
resources is adapted automatically as a single-skill package. Package hosting
therefore stays in the definition layer without requiring an Alan-specific
manifest for every public skill directory. Explicit `PackageMount` semantics are
the next layer and are introduced separately.

### Workspace — The Persistent Context

A **Workspace** is the persistent, stateful context in which an agent operates.
It gives the resolved agent definition its identity, memory, and working
environment.

```rust
pub struct WorkspaceRuntimeConfig {
    pub agent_config: AgentConfig,           // resolved runtime config from AgentRoot overlays
    pub workspace_id: String,                // identity
    pub workspace_root_dir: Option<PathBuf>, // workspace root used for tool cwd
    pub workspace_alan_dir: Option<PathBuf>, // `.alan` state directory
    pub resume_rollout_path: Option<PathBuf>, // session restore point
}
```

**Workspace directory layout:**

```
{home}/.alan/
├── agent/
│   ├── agent.toml          # global base agent config
│   ├── persona/            # global base persona overlays
│   ├── skills/             # global base skills
│   └── policy.yaml         # optional global base policy override
├── agents/
│   └── <name>/
│       ├── agent.toml      # global named agent config
│       ├── persona/
│       ├── skills/
│       └── policy.yaml
├── host.toml               # daemon/client host config
├── models.toml             # optional global model overlay catalog
├── sessions/
│   └── <session-id>.json   # daemon session bindings (workspace + governance)

{workspace_root}/.alan/
├── state.json              # workspace state (status, config, current session), when persisted
├── agent/
│   ├── agent.toml          # workspace base agent config
│   ├── persona/            # workspace base persona overlays
│   ├── skills/             # workspace base skills
│   └── policy.yaml         # optional workspace base policy override
├── agents/
│   └── <name>/
│       ├── agent.toml      # workspace named agent config
│       ├── persona/
│       ├── skills/
│       └── policy.yaml
├── memory/
│   └── MEMORY.md           # long-term knowledge
├── sessions/
│   └── rollout-*.jsonl     # persisted rollout files

{workspace_root}/.alan/sessions/
└── rollout-*.jsonl         # current + archived session rollouts
```

**Key properties:**
- **Persistent** — survives restarts, maintains identity across sessions
- **Self-contained** — workspace state and tool state live under the workspace `.alan` directory; session bindings are tracked by daemon metadata
- **Composable** — different Agents can be mounted into the same Workspace

### AgentInstance — The Running Process

An **AgentInstance** is the running runtime process bound to one resolved agent
definition and one workspace at a time.

**Key properties:**
- **Fresh launch semantics** — startup is derived from the resolved definition, not from hidden parent prompt inheritance
- **Supervised by the host layer** — lifecycle is owned by the daemon/CLI layer, not by `alan-runtime` alone
- **Distinct from overlay resolution** — parent/child instance relations are runtime supervision, not definition ancestry

### Session — The Computation

A **Session** is a single, bounded execution inside an `AgentInstance`. It
represents one conversation or task, limited by the LLM's context window.

**Key properties:**
- **Bounded** — constrained by the context window; when full, start a new session
- **Archivable** — completed sessions are saved as rollouts for replay or forking
- **One active session per workspace** at any time; others are paused or archived

---

## Policy Model (Policy Over Sandbox V2)

Alan uses policy-as-code as the only decision layer for tool governance.

1. **Policy gate (`PolicyEngine`)**: per-call decision `allow | deny | escalate` based on tool name, capability, and command patterns.
2. **Sandbox backend**: the current `workspace_path_guard` backend is a best-effort execution guard for workspace paths and shell shape checks, not a strict OS sandbox.

`escalate` always maps to `Event::Yield` and waits for `Op::Resume`. There is no `approval_policy` downgrade branch.

Policy file resolution is:

1. `governance.policy_path`, if provided
2. the highest-precedence existing `policy.yaml` in the resolved `AgentRoot` chain
3. builtin profile defaults

When a policy file is found, it replaces the builtin profile rule set for that session. There is no implicit merge with builtin rules.

Detailed current behavior: [`governance_current_contract.md`](./governance_current_contract.md).  
Target V2 design: [`policy_over_sandbox.md`](./policy_over_sandbox.md).

---

## Turing Machine Mapping

| TM Concept              | Alan Implementation                                          |
| ----------------------- | ------------------------------------------------------------ |
| **Program**             | Resolved `AgentRoot` definition consumed as runtime config   |
| **Tape**                | `Tape` — messages, context items, conversation summary       |
| **Head**                | Current turn — reads tape, produces output                   |
| **Transition Function** | LLM generation — maps (state, input) → (action, new state)   |
| **State**               | `Session` — holds tape, tools, skills, and runtime config    |
| **Machine**             | `AgentInstance` running against a `Workspace`                |
| **Alphabet**            | Messages (user/assistant/tool) and tool calls                |
| **Halt**                | No more tool calls, final text response emitted              |

---

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        Clients                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │   TUI    │  │  Native  │  │   API    │              │
│  │  (Bun)   │  │ (SwiftUI)│  │ (HTTP/WS)│              │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘              │
└───────┼─────────────┼─────────────┼─────────────────────┘
        └─────────────┴─────────────┘
                      │
              ┌───────▼─────────────────────────┐
              │         alan daemon             │  ← Workspace lifecycle & hosting
              │ runtime_manager/session_store   │
              └───────┬─────────────────────────┘
                      │ manages
        ┌─────────────┼─────────────┐
        │             │             │
   ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐
   │  Agent   │ │  Agent   │ │  Agent   │  ← Running instances bound to workspaces
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
| `alan-llm`      | Pluggable LLM adapters — Google Gemini GenerateContent API, OpenAI Responses API, OpenAI Chat Completions API, OpenAI Chat Completions API-compatible, Anthropic Messages API (+ OpenRouter via adapter) |
| `alan-runtime`  | Core engine — session, tape, agent loop, tool registry, skills   |
| `alan-tools`    | Builtin tool implementations (`read_file`, `bash`, `grep`, etc.) |
| `alan`          | Unified CLI + daemon — workspace lifecycle, HTTP/WS API, session mgmt |

---

## Design Principles

1. **Stateless Agent, Stateful Workspace** — Clean separation between reusable computation logic and persistent identity/context.

2. **Checkpointed Reasoning** — Every thought, action, and observation is durably recorded in the session rollout.

3. **Generic Core** — `alan-runtime` is provider-agnostic, domain-agnostic, and hosting-agnostic. The same runtime powers different agents, workspaces, and deployment targets.

4. **Skills-First, Extension-Ready** — Workflow intelligence lives in skills; pluggable system capabilities live in extensions behind stable contracts.

5. **Bounded Sessions** — Context windows are finite. Instead of fighting this constraint, Alan embraces it: sessions are discrete, archivable units that can be summarized, forked, and resumed.
