# Skills & Tools — Extending the Machine

> Status: current tool behavior plus accepted V2 governance direction.
> The authoritative current governance contract lives in
> [`governance_current_contract.md`](./governance_current_contract.md).

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
2. **Sandbox backend**: the current `workspace_path_guard` backend is a best-effort execution guard, not a strict OS sandbox.

When policy returns `escalate`, runtime emits `Event::Yield` and waits for `Op::Resume`. This path is explicit and does not depend on session-level approval toggles.

Current contract: [governance_current_contract.md](./governance_current_contract.md).  
Target V2 design and policy file format: [policy_over_sandbox.md](./policy_over_sandbox.md).

### Steering During Tool Execution

`Op::Input` is treated as steering input. During a tool batch, runtime checks in-band steering after each tool call. If steering exists, remaining calls are marked as skipped and the steering message is injected before the next LLM generation.

### Filesystem Sandbox

The current `Sandbox` ([sandbox.rs](../crates/runtime/src/tools/sandbox.rs)) enforces workspace-only filesystem access:

- **Path validation** — `is_in_workspace()` canonicalizes paths (via `dunce`) and checks containment
- **New file support** — walks parent directories to validate paths that don't exist yet
- **Read / Write / Exec / ListDir** — all operations check workspace containment before proceeding
- **Protected subpaths (current path-guard backend)** — file writes are blocked by default under `.git`, `.alan`, and `.agents`, and process path references into those subpaths are blocked conservatively because shell commands cannot be proven read-only
- **Plain shell commands only** — the workspace path guard rejects shell variable, command, brace, and glob expansion, rejects shell control flow, rejects common wrapper forms like `env`, `command`, `builtin`, `exec`, `time`, `nice`, `nohup`, `timeout`, `stdbuf`, and `setsid`, rejects direct nested evaluators like `eval`, `.`, and `sh/bash/python -c`, rejects direct opaque command dispatchers like `xargs` and `find -exec`, and rejects a curated set of common direct script interpreters like `python file.py`, `bash script.sh`, and `awk -f script.awk` because script bodies and dynamic child paths cannot be validated safely before execution. It validates explicit path-like argv references and redirection targets, but it does not infer utility-specific operand roles for arbitrary bare tokens. It also does not inspect arbitrary program-internal writes or dispatch, including commands that mutate private state without an explicit path operand such as `git init`, `git add`, or `git config --local`, utility actions like `find -delete`, or utility-specific script/DSL modes or opaque recipe/script execution inside build or task runners, such as `sed -f`.
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

### Capability-Package Sources

Alan now resolves skills through one `ResolvedCapabilityView` instead of a
separate `repo/user/builtin` loading path. The current capability sources are:

| Source         | Location / Form                                         | Role                          |
| -------------- | ------------------------------------------------------- | ----------------------------- |
| **Built-in**   | Embedded first-party package assets                     | Core Alan capabilities        |
| **User public skills** | `~/.agents/skills/`                              | Zero-conversion public installs |
| **User roots** | `~/.alan/agent/skills/` and `~/.alan/agents/<name>/skills/` | Alan-native cross-project capability sources |
| **Workspace public skills** | `<workspace>/.agents/skills/`              | Zero-conversion workspace installs |
| **Workspace roots** | `.alan/agent/skills/` and `.alan/agents/<name>/skills/` | Alan-native project/workspace capability sources |

Within the user and workspace sources, Alan follows the resolved `AgentRoot`
overlay chain, and later roots override earlier ones when skill IDs collide.

Overlay order is:

- Default workspace agent: `~/.alan/agent -> <workspace>/.alan/agent`
- Named agent: `~/.alan/agent -> <workspace>/.alan/agent -> ~/.alan/agents/<name> -> <workspace>/.alan/agents/<name>`

A standards-compatible skill directory with `SKILL.md` and optional
`scripts/`, `references/`, or `assets/` is adapted automatically into a
single-skill package. This keeps public skill compatibility without requiring a
custom `package.toml`.

Direct installation can therefore stay zero-conversion:

```text
~/.agents/skills/<skill-name>/SKILL.md
<workspace>/.agents/skills/<skill-name>/SKILL.md
```

`alan init` creates `<workspace>/.agents/skills/` for this workflow, and the
first-run setup wizard creates `~/.agents/skills/` alongside the canonical
global agent config.

### Package Mounts

Package discovery and package exposure are separate. Roots discover packages
from `skills/`, then `agent.toml` decides how each package is exposed:

```toml
[[package_mounts]]
package = "builtin:alan-plan"
mode = "always_active"

[[package_mounts]]
package = "skill:release-checklist"
mode = "explicit_only"
```

Supported modes are:

- `always_active`: catalog-visible and active every turn
- `discoverable`: catalog-visible and activated on demand
- `explicit_only`: hidden from the catalog but activatable by explicit `$skill`
- `internal`: not exposed through the current skill prompt/runtime

The default global base agent root mounts the built-in packages as
`always_active`. When no explicit mount is provided for a discovered root-backed
or public single-skill package, it defaults to `discoverable`.

### Built-In Packages

Three first-party packages are embedded as built-in assets and included in the
resolved capability view:

| Skill      | Purpose                                                       |
| ---------- | ------------------------------------------------------------- |
| **memory** | Persistent knowledge across sessions (`.alan/memory/`)        |
| **plan**   | Structured execution plans for complex tasks (`.alan/plans/`) |
| **workspace-manager** | Workspace lifecycle operations and recovery guidance |

Current built-in package ids are `builtin:alan-memory`,
`builtin:alan-plan`, and `builtin:alan-workspace-manager`.

Source: [skills/memory/SKILL.md](../crates/runtime/skills/memory/SKILL.md), [skills/plan/SKILL.md](../crates/runtime/skills/plan/SKILL.md), [skills/workspace-manager/SKILL.md](../crates/runtime/skills/workspace-manager/SKILL.md)

These are exposed through the same package + mount model as every other
capability. They remain embedded assets, but they are mounted from the default
global base agent root instead of a separate runtime-only builtin-skill path.

You can inspect the resolved view directly from the CLI:

```bash
alan skills list
alan skills packages
```

### Triggering

Skills are activated according to the resolved mount mode. The injector ([injector.rs](../crates/runtime/src/skills/injector.rs)):

1. Extracts `$skill-name` / `$skill_name` patterns from input
2. Loads full skill content on demand
3. Injects skill instructions + resource listings into the prompt

`always_active` packages are injected by default. `discoverable` packages appear
in the skills catalog and can be activated on demand. `explicit_only` packages
skip the catalog but still respond to exact `$skill-name` mentions.

A skills catalog is also rendered into the system prompt so the LLM can reference available skills.

### Module Structure

```
crates/runtime/src/skills/
├── mod.rs             # exports + built-in skill/package assets
├── types.rs           # SkillMetadata, PortableSkill, CapabilityPackage, PackageMount, ...
├── loader.rs          # Filesystem scanning + SKILL.md parsing
├── capability_view.rs # build ResolvedCapabilityView from package sources
├── registry.rs        # SkillsRegistry — mount-aware lookup, listing, matching
└── injector.rs        # $mention extraction, prompt injection, catalog rendering
```

---

## Design Principles

1. **Generic Core** — `alan-runtime` defines `Tool` trait and `ToolRegistry` but contains zero tool implementations. The same core powers any tool set.

2. **Skills over Plugins** — Capabilities are Markdown instructions that shape behavior, not compiled code that extends the runtime. Adding a skill requires no recompilation.

3. **Self-Sufficient Skills** — Skills carry their own scripts and references. They extend capability through `bash` + existing tools, not through new native code.

4. **No MCP** — No external protocol dependencies. Tools are direct Rust trait implementations; skills are local filesystem documents.

5. **Policy Over Sandbox** — policy decides intent, and the current sandbox backend provides a best-effort execution guard.

6. **Path-Based Filesystem Isolation** — simple, portable workspace containment without OS-specific mechanisms; trades maximum isolation for zero external dependencies.
