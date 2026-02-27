# Skills & Tools — Extending the Machine

## Overview

In the AI Turing Machine model, the core runtime is a generic state-transition engine. **Tools** are the side-effect interface — how the agent operates on the external world. **Skills** are dynamic instruction extensions — Markdown documents that reshape the agent's behavior at runtime. Together they let Alan remain a small, generic core while supporting arbitrarily rich capabilities.

| Concept     | TM Role               | Implementation                                        |
| ----------- | --------------------- | ----------------------------------------------------- |
| **Tools**   | Side effects          | `Tool` trait in `alan-runtime`, impls in `alan-tools` |
| **Skills**  | Instruction extension | Markdown + YAML, loaded into prompt context           |
| **Sandbox** | Boundary constraint   | Workspace-only path enforcement in `alan-runtime`     |

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

Every tool implements a single trait ([registry.rs](file:///Users/morris/Developer/Alan/crates/runtime/src/tools/registry.rs)):

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;            // JSON Schema
    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult;
    fn capability(&self, args: &Value) -> ToolCapability;  // read / write / execute
    fn timeout_secs(&self) -> usize;                 // default 120s
}
```

### ToolRegistry

`ToolRegistry` manages tool lifecycle: registration, lookup, schema validation, and execution. It generates `ToolDefinition` objects for LLM function-calling APIs and enforces JSON Schema validation before dispatch.

### ToolContext

Each tool invocation receives a `ToolContext` ([context.rs](file:///Users/morris/Developer/Alan/crates/runtime/src/tools/context.rs)) carrying:

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

### Tool Catalog

| Tool         | Capability | Description                                                |
| ------------ | ---------- | ---------------------------------------------------------- |
| `read_file`  | Read       | Read file contents, supports offset/limit, image detection |
| `write_file` | Write      | Write file, auto-creates parent directories                |
| `edit_file`  | Write      | Search-and-replace editing                                 |
| `bash`       | Execute    | Shell command execution (120s timeout)                     |
| `grep`       | Read       | Recursive regex search across files                        |
| `glob`       | Read       | File path pattern matching                                 |
| `list_dir`   | Read       | Directory listing, directories sorted first                |

All implementations live in [alan-tools/src/lib.rs](file:///Users/morris/Developer/Alan/crates/tools/src/lib.rs).

### Sandbox

The `Sandbox` ([sandbox.rs](file:///Users/morris/Developer/Alan/crates/runtime/src/tools/sandbox.rs)) enforces workspace-only access:

- **Path validation** — `is_in_workspace()` canonicalizes paths (via `dunce`) and checks containment
- **New file support** — walks parent directories to validate paths that don't exist yet
- **Read / Write / Exec / ListDir** — all operations check workspace containment before proceeding
- **No OS-level sandboxing** — no Landlock, Seatbelt, or container isolation; purely path-based

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

| Scope      | Location                        | Purpose                       |
| ---------- | ------------------------------- | ----------------------------- |
| **Repo**   | `.alan/skills/` in project root | Project-specific capabilities |
| **User**   | `~/.config/alan/skills/`        | Personal cross-project skills |
| **System** | Compiled into binary            | Always-on core behaviors      |

Higher-priority scopes override lower ones when skill IDs collide.

### System Skills

Two skills are embedded at compile time and always available:

| Skill      | Purpose                                                       |
| ---------- | ------------------------------------------------------------- |
| **memory** | Persistent knowledge across sessions (`.alan/memory/`)        |
| **plan**   | Structured execution plans for complex tasks (`.alan/plans/`) |

Source: [skills/memory/SKILL.md](file:///Users/morris/Developer/Alan/crates/runtime/skills/memory/SKILL.md), [skills/plan/SKILL.md](file:///Users/morris/Developer/Alan/crates/runtime/skills/plan/SKILL.md)

### Triggering

Skills are activated by `$skill-name` mentions in user input. The injector ([injector.rs](file:///Users/morris/Developer/Alan/crates/runtime/src/skills/injector.rs)):

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

5. **Path-Based Sandbox** — Simple, portable workspace containment without OS-specific mechanisms. Trades maximum isolation for zero external dependencies.
