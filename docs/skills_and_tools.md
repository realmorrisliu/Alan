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
├── skill.yaml            # Optional: Alan-native machine metadata for this skill
├── package.yaml          # Optional: package-level defaults for Alan-native metadata
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

Declared trigger behavior is now deterministic in runtime:

- explicit `$skill-id` mentions always win when the skill is visible and
  available
- `triggers.explicit` can define additional explicit aliases such as `$ship-it`
- `triggers.keywords` and `triggers.patterns` can auto-activate discoverable
  skills without a model-side classifier
- `triggers.negative_keywords` suppresses automatic activation, but does not
  override an explicit user mention

### Alan Sidecar Metadata

Alan also supports optional machine-readable sidecars that do not change the
`SKILL.md` portability contract:

- `skill.yaml`: skill-specific Alan-native metadata
- `package.yaml`: package-level defaults applied before the skill sidecar

Current precedence is:

1. `SKILL.md` frontmatter as the compatibility baseline
2. `package.yaml` `skill_defaults`
3. `skill.yaml`

This is fail-open: when sidecar files are absent, discovery and activation still
work from `SKILL.md` alone. Invalid sidecars are recorded as non-fatal load
errors and Alan skips only the broken overlay while keeping any other valid
sidecar layers.

Alan currently uses sidecar runtime metadata for two product behaviors:

- `runtime.permission_hints` can be attached to active-skill confirmation and
  approval surfaces as advisory context before privileged actions
- unavailable skills now render remediation guidance instead of a bare
  unavailable label when declared dependencies such as required tools or minimum
  Alan version are missing

### Delegated Skill Execution

Alan now resolves a package-local execution contract for each discovered skill.

The current resolved states are:

- `inline`
- `delegate(target=...)`
- `unresolved(...)`

Execution metadata lives in Alan sidecars rather than `SKILL.md`, for example:

```yaml
runtime:
  execution:
    mode: delegate
    target: reviewer
```

Default inference is package-local and deterministic:

- if a package exports no child-agent roots, the skill resolves to `inline`
- if a skill id matches a child-agent export name, it resolves to `delegate`
- if a package exports exactly one skill and exactly one child-agent root, that
  skill resolves to `delegate`
- ambiguous package shapes do not guess; they resolve to `unresolved(...)`
  until explicit sidecar metadata is present

This keeps delegated execution strict and predictable.

When an active skill resolves to `delegate(target=...)`, top-level Alan
runtimes expose delegated invocation by default. Alan no longer injects the
full `SKILL.md` body into the parent prompt. Instead, the parent sees a
lightweight delegated-capability stub with:

- the resolved `skill_id`
- the resolved delegated `target`
- an explicit `invoke_delegated_skill` runtime tool contract

This keeps the parent-side context small and makes delegated execution an
explicit runtime-owned path instead of an inline prompt convention.

The delegated tool now launches the resolved package-local child-agent export
through `SpawnSpec` with a fresh child runtime and explicit handles only.
V1 keeps that default launch narrow: the child gets the workspace handle, but
it does not inherit parent tape, active skills, or plan state by default.

Alan still preserves a compatibility fallback for runtimes that do not expose
delegated invocation, for example child runtimes where nested delegated
execution is intentionally disabled in V1. In those runtimes, delegated skills
keep their inline `SKILL.md` instructions and surface a short runtime-fallback
note instead of a non-functional delegated tool path.

The delegated invocation path records a bounded invocation/result object in the
parent tape:

```json
{
  "skill_id": "repo-review",
  "target": "repo-review",
  "task": "Review the current diff for correctness and missing tests.",
  "result": {
    "status": "completed",
    "summary": "High-level delegated outcome",
    "structured_output": {
      "optional": "capability-specific data"
    }
  }
}
```

The parent consumes that bounded record instead of replaying the child's full
transcript into parent context.

The parent rollout keeps a richer out-of-band reference for debugging and
auditing. Its delegated tool-call record additionally stores a `child_run`
object with the child session id, rollout path when durable, and terminal
status. That metadata stays out of the parent tape by default, so future prompt
assembly does not absorb child rollout details while operators can still inspect
the child run separately when needed.

`alan skills list` and `alan skills packages` also surface each resolved
execution mode and flag unresolved delegated-package shapes with explicit
diagnostics.

If execution resolves to `unresolved(...)`, Alan also avoids injecting the full
body. The parent receives an execution-status stub that surfaces the unresolved
reason so ambiguous package shapes do not silently fall back to inline behavior.

### Progressive Disclosure

Alan now consumes `capabilities.disclosure` during prompt assembly instead of
leaving it schema-only.

```yaml
capabilities:
  disclosure:
    level2: details.md
    level3:
      references: ["quickstart.md"]
      scripts: ["scripts/check.sh"]
      assets: ["assets/template.txt"]
```

Runtime behavior:

- `level2` chooses the primary instruction document injected for the active
  skill. When omitted, Alan uses the `SKILL.md` body.
- `level3` lists package resources that may be expanded into the prompt when the
  skill is active.
- Relative resource references already mentioned in the active instruction text
  such as `references/guide.md` or `scripts/build.sh` are also resolved
  deterministically against the canonical `resource_root`.
- Prompt-cache invalidation tracks the concrete disclosed files, so edits to a
  referenced `details.md` or `references/*.md` file invalidate the active-skill
  render without requiring a full directory rescan.

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
`scripts/`, `references/`, `assets/`, `viewers/`, or child-agent roots under
`agents/` is adapted automatically into a
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

Mount overlays are merged by `package` id across the resolved root chain. Later
roots override earlier mount modes for the same package without discarding
unrelated mounts from lower-precedence roots.

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
2. Resolves a structured active-skill envelope for each selected skill
3. Loads full skill content on demand
4. Injects skill instructions together with stable path and package context

The active-skill envelope now carries:

- skill metadata (`id`, `package_id`, `mount_mode`)
- canonical `SKILL.md` path
- canonical package root and resource root when available
- availability state
- activation reason
- resolved execution state

Prompt injection consumes that structured shape instead of pathless Markdown
fragments. Active skill sections therefore include an `Alan Runtime Context`
block before the skill body so downstream resource resolution can be
deterministic.

`always_active` packages are injected by default. `discoverable` packages appear
in the skills catalog and can be activated on demand. `explicit_only` packages
skip the catalog but still respond to exact `$skill-name` mentions.

Skill availability is also filtered by frontmatter compatibility:

- `required_tools`
- `min_version`

If a skill fails those checks, Alan keeps the package in the resolved
definition layer but excludes the skill from runtime activation. Explicit
mentions then surface a concrete unavailable reason instead of silently
injecting an unusable skill.

A skills catalog is also rendered into the system prompt so the LLM can reference available skills.

Resource listings are now emitted relative to the envelope's `resource_root`
instead of bare filenames, for example `scripts/check.sh` rather than only
`check.sh`.

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
