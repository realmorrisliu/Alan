# Skills & Tools — Extending the Machine

> Status: this document aligns with the accepted V2 governance direction; sandbox backend upgrades are marked where still in migration.

## Overview

In the AI Turing Machine model, the core runtime is a generic state-transition engine. **Tools** are the side-effect interface — how the agent operates on the external world. **Skills** are dynamic instruction extensions — Markdown documents that reshape the agent's behavior at runtime. Together they let Alan remain a small, generic core while supporting arbitrarily rich capabilities.

| Concept     | TM Role               | Implementation                                        |
| ----------- | --------------------- | ----------------------------------------------------- |
| **Tools**   | Side effects          | `Tool` trait in `alan-runtime`, impls in `alan-tools` |
| **Skills**  | Instruction extension | Markdown + YAML, loaded into prompt context           |
| **Sandbox** | Boundary constraint   | Execution backend (filesystem/process/network limits)  |
| **Policy**  | Decision boundary     | `PolicyEngine` rules (`allow/deny/escalate`)           |

---

## Tool System

### Architecture

The runtime defines **what a tool looks like**; a separate crate provides **concrete implementations**. This keeps the core provider-agnostic and domain-agnostic.

```
alan-runtime (trait + registry)          alan-tools (implementations)
┌──────────────────────────────┐        ┌─────────────────────────────┐
│  Tool trait                  │◄───────│  ReadFileTool               │
│  ToolRegistry                │        │  WriteFileTool              │
│  ToolContext                 │        │  EditFileTool               │
│  Sandbox                     │        │  BashTool                   │
└──────────────────────────────┘        │  GrepTool                   │
                                        │  GlobTool                   │
                                        │  ListDirTool                │
                                        └─────────────────────────────┘
```

### Tool Trait

Every tool implements a single trait ([registry.rs](../crates/runtime/src/tools/registry.rs)):

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;            // JSON Schema
    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult;
    fn capability(&self, args: &Value) -> ToolCapability;  // read / write / network
    fn timeout_secs(&self) -> usize;                 // tool default timeout (seconds)
}
```

### ToolRegistry

`ToolRegistry` manages tool lifecycle: registration, lookup, schema validation, and execution. It generates `ToolDefinition` objects for LLM function-calling APIs and enforces JSON Schema validation before dispatch.

### ToolContext

Each tool invocation receives a `ToolContext` ([context.rs](../crates/runtime/src/tools/context.rs)) carrying:

| Field         | Purpose                                        |
| ------------- | ---------------------------------------------- |
| `cwd`         | Working directory for relative path resolution |
| `scratch_dir` | Temporary storage for tool intermediates       |
| `config`      | Shared runtime configuration (`Arc<Config>`)   |

### Built-in Tool Profiles

`alan-tools` ships 7 built-ins with layered profiles:

- **Core (default)**: `read_file`, `write_file`, `edit_file`, `bash`
- **Read-only exploration**: `read_file`, `grep`, `glob`, `list_dir`
- **All built-ins**: core + exploration tools (7 total)

### Virtual Tools

Runtime injects three virtual tools into the LLM toolset for planning and human-in-the-loop control:

- `request_confirmation` — pause execution and emit `Event::Yield` with kind `confirmation`
- `request_user_input` — pause execution and emit `Event::Yield` with kind `structured_input`
- `update_plan` — update in-memory plan metadata before continuing in the current turn

These are implemented in `runtime/virtual_tools.rs` and are handled by the runtime itself, not `alan-tools`.

### Tool Catalog

| Tool         | Capability | Description                                                |
| ------------ | ---------- | ---------------------------------------------------------- |
| `read_file`  | Read       | Read file contents, supports offset/limit, image detection |
| `write_file` | Write      | Write file, auto-creates parent directories                |
| `edit_file`  | Write      | Search-and-replace editing                                 |
| `bash`       | Dynamic    | Shell command execution with command-based capability classification |
| `grep`       | Read       | Recursive regex search across files                        |
| `glob`       | Read       | File path pattern matching                                 |
| `list_dir`   | Read       | Directory listing, directories sorted first                |

All implementations live in [alan-tools/src/lib.rs](../crates/tools/src/lib.rs).

`bash` exposes a `timeout` argument in schema (1–300, default 60), and the tool-level default timeout is currently 300 seconds (`timeout_secs`).

### Tool Governance: Policy First, Sandbox Enforced

Alan V2 governance is:

1. **Policy gate**: per-call decision (`allow`, `deny`, `escalate`).
2. **Sandbox backend**: enforces execution boundaries for calls that are allowed to run.

When policy returns `escalate`, runtime emits `Event::Yield` and waits for `Op::Resume`. This path is explicit and does not depend on session-level approval toggles.

Detailed model and policy file format: [policy_over_sandbox.md](./policy_over_sandbox.md).

### Steering During Tool Execution

`Op::Input` is treated as steering input. During a tool batch, runtime checks in-band steering after each tool call. If steering exists, remaining calls are marked as skipped and the steering message is injected before the next LLM generation.

### Filesystem Sandbox

The current `Sandbox` ([sandbox.rs](../crates/runtime/src/tools/sandbox.rs)) enforces workspace-only filesystem access:

- **Path validation** — `is_in_workspace()` canonicalizes paths (via `dunce`) and checks containment
- **New file support** — walks parent directories to validate paths that don't exist yet
- **Read / Write / Exec / ListDir** — all operations check workspace containment before proceeding
- **No OS-level sandboxing (current state)** — no Landlock, Seatbelt, or container isolation; purely path-based

V2 direction: keep path-based checks as baseline backend, then add optional OS-level sandbox backends and protected subpaths under writable roots.

---

## Skill System

### Design

Skills are self-contained, filesystem-based capability packages. Each skill is a directory with a required `SKILL.md` file and optional supporting resources:

```
my-skill/
├── SKILL.md              # Required: YAML frontmatter + Markdown instructions
├── scripts/              # Optional: executable code the agent can invoke via bash
├── references/           # Optional: reference documentation
└── assets/               # Optional: templates, resources
```

### SKILL.md Format

```yaml
---
name: skill-name
description: What this skill does and when to use it
metadata:
  short-description: Brief one-liner
  tags: ["tag1", "tag2"]
capabilities:
  required_tools: [read_file, bash]
  triggers:
    keywords: [keyword1, keyword2]
    patterns: ["regex.*pattern"]
---

# Instructions

Step-by-step guidance for the agent...
```

### Three-Level Scope

Skills are discovered from three sources, in priority order (highest first):

| Scope      | Location                                              | Purpose                       |
| ---------- | ----------------------------------------------------- | ----------------------------- |
| **Repo**   | `.alan/skills/` | Project/workspace-specific capabilities |
| **User**   | `~/.alan/skills/`                                     | Personal cross-project skills |
| **System** | Compiled into binary                                  | Always-on core behaviors      |

Higher-priority scopes override lower ones when skill IDs collide.

### System Skills

Three skills are embedded at compile time and always available:

| Skill      | Purpose                                                       |
| ---------- | ------------------------------------------------------------- |
| **memory** | Persistent knowledge across sessions (`.alan/memory/`)        |
| **plan**   | Structured execution plans for complex tasks (`.alan/plans/`) |
| **workspace-manager** | Workspace lifecycle operations and recovery guidance |

Source: [skills/memory/SKILL.md](../crates/runtime/skills/memory/SKILL.md), [skills/plan/SKILL.md](../crates/runtime/skills/plan/SKILL.md), [skills/workspace-manager/SKILL.md](../crates/runtime/skills/workspace-manager/SKILL.md)

### Triggering

Skills are activated by `$skill-name` mentions in user input. The injector ([injector.rs](../crates/runtime/src/skills/injector.rs)):

1. Extracts `$skill-name` / `$skill_name` patterns from input
2. Loads full skill content on demand
3. Injects skill instructions + resource listings into the prompt

A skills catalog is also rendered into the system prompt so the LLM can reference available skills.

### Module Structure

```
crates/runtime/src/skills/
├── mod.rs        # init(), list_skills(), system skill constants
├── types.rs      # SkillMetadata, Skill, SkillScope, SkillCapabilities, ...
├── loader.rs     # Filesystem scanning + SKILL.md parsing
├── registry.rs   # SkillsRegistry — load, reload, lookup, match
└── injector.rs   # $mention extraction, prompt injection, catalog rendering
```

---

## Design Principles

1. **Generic Core** — `alan-runtime` defines `Tool` trait and `ToolRegistry` but contains zero tool implementations. The same core powers any tool set.

2. **Skills over Plugins** — Capabilities are Markdown instructions that shape behavior, not compiled code that extends the runtime. Adding a skill requires no recompilation.

3. **Self-Sufficient Skills** — Skills carry their own scripts and references. They extend capability through `bash` + existing tools, not through new native code.

4. **No MCP** — No external protocol dependencies. Tools are direct Rust trait implementations; skills are local filesystem documents.

5. **Policy Over Sandbox** — policy decides intent, sandbox enforces execution boundaries.

6. **Path-Based Filesystem Isolation** — simple, portable workspace containment without OS-specific mechanisms; trades maximum isolation for zero external dependencies.
