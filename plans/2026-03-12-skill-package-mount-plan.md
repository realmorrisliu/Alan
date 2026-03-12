# Alan Skill / Package / Mount Model Plan (2026-03-12)

## Context

Alan's current skill system is functional, but it is still optimized for the older
`repo/user/system skill` model:

- repo skills are loaded from `{workspace}/.alan/skills/`
- user skills are loaded from `~/.alan/skills/`
- built-in system skills are hard-coded into the binary as embedded `SKILL.md` strings

That model is no longer ideal once Alan moves toward the newer agent-hosting design:

- `AgentRoot`
- `AgentInstance`
- `SpawnSpec`
- explicit mounts and explicit child-agent spawning

It is also not enough for richer public skill ecosystems such as Anthropic's `skills` repository
and the Agent Skills standard, where a skill is a portable directory that can carry:

- `SKILL.md`
- `scripts/`
- `references/`
- `assets/`

More complex first-party capabilities such as `skill-creator` go beyond a single portable skill.
They may also want:

- multiple related skills
- internal child agents
- evaluation scripts
- viewers or other bundled resources

Alan therefore needs a unified skill design that does two things at once:

1. remain compatible with portable public skills
2. support richer Alan-native capability bundles aligned with `AgentRoot` and `SpawnSpec`

## Problem Statement

If Alan keeps only the old `repo/user/system skill` abstraction, it will run into three limits:

1. built-in "system skills" remain a special implementation path instead of using the same host
   model as future first-party capabilities
2. richer capabilities such as `skill-creator` have nowhere natural to expose internal agents or
   extra resources
3. public skills become awkward to install if Alan requires a custom manifest or Alan-specific
   packaging before they can be used

The target design must avoid all three problems.

## Goals

1. Define one unified skill system aligned with the new agent-runtime model.
2. Support direct installation of public skills that follow the Agent Skills standard.
3. Replace today's special-case "system skill" path with the same package/mount model used by
   future first-party capabilities.
4. Allow richer first-party capabilities to export:
   - portable skills
   - internal child agents
   - scripts, references, assets, and viewers
5. Make `AgentRoot` the place that decides which capabilities are mounted and visible.
6. Preserve progressive disclosure so Alan does not eagerly load whole capability trees into
   prompt context.

## Non-Goals

1. Do not implement child-agent runtime orchestration in this plan; that belongs to the
   `AgentRoot / AgentInstance / SpawnSpec` work.
2. Do not redefine the external Agent Skills standard.
3. Do not force all public skills to adopt an Alan-specific manifest.
4. Do not require every capability package to export child agents.

## Design Principles

1. Keep portable skills portable.
2. Put Alan-specific metadata in package or mount metadata, not in `SKILL.md`.
3. Let `AgentRoot` decide mounts, visibility, and defaults.
4. Treat built-in first-party capabilities as packages, not as a special skill kind.
5. Keep package hosting in the definition layer and child-agent execution in the runtime layer.
6. Preserve progressive disclosure and on-demand loading.

## Core Model

Alan should standardize on three layers:

### 1. Portable Skill

A `PortableSkill` is the public, standards-compatible skill unit.

It should stay as close as possible to the Agent Skills model:

- a directory rooted at a `SKILL.md`
- optional `scripts/`
- optional `references/`
- optional `assets/`

This is the installable unit Alan should accept from public skill ecosystems without requiring
conversion.

### 2. Capability Package

A `CapabilityPackage` is Alan's internal packaging and distribution unit.

A package may export:

- one or more portable skills
- zero or more named child-agent roots
- package-scoped resources such as:
  - scripts
  - references
  - assets
  - viewers

This is the right abstraction for:

- built-in first-party capabilities such as `memory`, `plan`, and `workspace-manager`
- richer public or private capabilities such as `skill-creator`

### 3. Package Mount

A `PackageMount` is how an `AgentRoot` makes a package available.

The mount decides:

- which package is mounted
- which exports are visible
- which exports are always active
- which exports are discoverable but not auto-active
- which exports are internal only

The package is passive. The agent root decides how the package is used.

## Relationship to the Agent Runtime Plan

This skill model is designed to sit cleanly on top of the `AgentRoot / AgentInstance / SpawnSpec`
model:

- `CapabilityPackage` belongs to the definition layer
- `PackageMount` belongs to `AgentRoot` resolution
- `AgentInstance` sees only a resolved capability view
- child agents exported by a package are executed via `SpawnSpec`, not via prompt-level persona
  cloning

This keeps the boundary clean:

- packages define capabilities
- mounts expose capabilities
- runtime executes capabilities

## External Compatibility Model

Alan should be able to install public skills directly.

### Key Rule

Any standards-compatible skill directory should automatically be treated as a valid single-skill
package.

That means:

- if a directory contains a valid `SKILL.md`
- Alan can load it without requiring `bundle.toml`, `package.toml`, or other Alan-specific
  packaging metadata

This is the most important compatibility rule because it keeps Alan open to public skill
ecosystems.

### Alan-Specific Metadata

Alan-specific semantics should not live in `SKILL.md`, for example:

- `always_active`
- `discoverable`
- `internal_only`
- `spawn_only`
- mount defaults

Those belong in:

- package manifest metadata
- mount metadata
- agent-root config

### Progressive Disclosure

Alan should preserve the same basic progressive disclosure model used by public skill systems:

1. load lightweight catalog metadata first
2. load full `SKILL.md` only when the skill becomes relevant
3. load package resources only on demand

Capability packages must not break this rule.

## Filesystem Model

Alan should support both package-native and standards-native filesystem layouts.

### Built-in Packages

Suggested built-in layout:

```text
crates/runtime/bundles/
  alan-memory/
    package.toml
    skills/
      memory/
        SKILL.md
  alan-plan/
    package.toml
    skills/
      plan/
        SKILL.md
  alan-workspace-manager/
    package.toml
    skills/
      workspace-manager/
        SKILL.md
```

### Home and Workspace Packages

```text
~/.alan/packages/<package-id>/
<workspace>/.alan/packages/<package-id>/
```

### Standards-Compatible Public Skills

Alan should also recognize standard skill directories directly, for example:

```text
~/.agents/skills/<skill-id>/
<workspace>/.agents/skills/<skill-id>/
```

Each such directory is automatically adapted as a single-skill package.

### Agent-Local Skills

An `AgentRoot` may also provide local skills directly:

```text
agent-root/
  skills/
    my-local-skill/
      SKILL.md
```

These should behave like local package exports owned by that root.

## Package Contract

### Minimal Case: No Package Manifest Required

If a capability package is just a single public portable skill, no Alan-specific manifest should
be required.

Alan should infer a single-skill package automatically.

### Extended Case: Package Manifest

A package manifest is only needed when a capability goes beyond the minimal portable-skill case.

Examples:

- multiple exported skills
- child-agent exports
- internal-only exports
- package metadata such as title, description, icon, or version
- richer package resource layout

Suggested manifest file:

```text
package.toml
```

The exact schema can be defined later, but it should stay package-level and must not leak into the
portable-skill format.

## Export Types

Packages should support at least these export categories:

### Skills

Portable skills exposed to the agent as discoverable or active capabilities.

### Agents

Named child-agent roots that can be spawned via the runtime's child-agent APIs.

### Resources

Package-scoped resources used by skills or agents:

- scripts
- references
- assets
- viewers

Resources are not directly "skills". They are support material for exported skills and agents.

## Mount Contract

`AgentRoot` should mount packages explicitly.

Illustrative shape:

```toml
[[mounts]]
source = "builtin:alan-memory"

[[mounts]]
source = "builtin:alan-plan"

[[mounts]]
source = "builtin:alan-workspace-manager"

[[mounts]]
source = "home:skill-creator"
```

The important semantic rule is:

- packages do not become active just because they exist
- an agent root decides which packages are mounted

### Mount Visibility Modes

The first version should support at least:

- `always_active`
- `discoverable`
- `explicit_only`
- `internal`

This gives enough expressive power to model:

- today's built-in always-on skills
- public optional skills
- internal-only grader/analyzer child agents

## Resolved Capability View

After `AgentRoot` resolution and package mounting, runtime should operate on a resolved capability
view rather than on raw package directories.

That resolved view should contain:

- skill catalog metadata
- active skill set
- discoverable skill set
- child-agent registry
- resource locators

This gives the runtime one stable abstraction to consume regardless of whether a capability came
from:

- an agent-local skill directory
- a home-level public skill
- a workspace package
- a built-in first-party package

## Upgrading Current System Skills

Today's built-in `memory`, `plan`, and `workspace-manager` skills should be upgraded into
built-in first-party packages.

Suggested direction:

- `memory` becomes `builtin:alan-memory`
- `plan` becomes `builtin:alan-plan`
- `workspace-manager` becomes `builtin:alan-workspace-manager`

Then:

- the default global base agent root mounts them
- the runtime no longer has a separate "system skill" code path
- first-party built-ins and future richer packages share one host model

This is the cleanest path to unifying old system skills with the new agent-runtime design.

## Example: `skill-creator`

Anthropic's `skill-creator` should eventually map to Alan as a capability package, not just a
single embedded skill.

Illustrative shape:

```text
skill-creator/
  package.toml
  skills/
    skill-creator/
      SKILL.md
  agents/
    grader/
      agent.toml
    analyzer/
      agent.toml
  scripts/
  references/
  assets/
  eval-viewer/
```

This package would export:

- one user-facing portable skill: `skill-creator`
- internal child agents such as `grader` and `analyzer`
- package resources used during evaluation and review

This is exactly the kind of richer capability that the old `repo/user/system skill` split cannot
model cleanly.

## Desired End State

By the end of this work:

- Alan accepts public portable skills without requiring repackaging
- built-in first-party skills are hosted as built-in packages
- agent roots mount packages explicitly
- package exports can include both skills and child-agent roots
- the runtime consumes a resolved capability view instead of multiple ad hoc sources
- there is no separate hard-coded "system skill" implementation path

## Phase Plan

### PR1: Define the Package / Mount Model

Goal: formalize the abstractions without changing all loaders yet.

Changes:

- define `PortableSkill`, `CapabilityPackage`, `PackageMount`, and `ResolvedCapabilityView`
- define minimal compatibility rules for standards-compatible public skills
- define the rule that a valid public skill directory is a valid single-skill package

### PR2: Introduce Package Hosting and Resolution

Goal: add package-native loading on the definition side.

Changes:

- add package discovery for:
  - built-in packages
  - `~/.alan/packages/`
  - `<workspace>/.alan/packages/`
- add adaptation of standards-compatible public skill directories into single-skill packages
- add resolved capability view assembly

### PR3: Upgrade Built-In System Skills to Built-In Packages

Goal: remove the current hard-coded system-skill path.

Changes:

- move `memory`, `plan`, and `workspace-manager` into built-in packages
- mount them from the default global base agent root
- remove direct runtime special-casing for embedded `SKILL.md` constants

### PR4: Add Package Mounts to Agent Roots

Goal: let `AgentRoot` decide which capability packages are visible and active.

Changes:

- add package mount config to `agent.toml`
- add visibility and activation modes
- make local agent-root skills behave as root-owned capability exports

### PR5: Add Package Export Support for Child Agents and Rich Resources

Goal: support richer first-party capabilities such as `skill-creator`.

Changes:

- add child-agent exports to package resolution
- add package resource locators for scripts/references/assets/viewers
- keep resource loading on demand

### PR6: Align Docs and Installer UX

Goal: make the new model discoverable and safe for public-skill installation.

Changes:

- update README and architecture docs
- document package vs skill vs mount semantics
- document the public-skill compatibility path
- update install/import UX to prefer zero-conversion paths for standard public skills

## Open Questions

1. Should Alan prefer `~/.agents/skills` or `~/.alan/packages` when the same skill id appears in
   both places?
2. Should agent-local `skills/` be treated as an implicit local package or as a separate export
   source merged into the resolved capability view?
3. How should package versioning work for built-in packages versus installed public skills?
4. Should package manifests support dependency references between packages in the first version, or
   should mounts stay flat?
5. How should mount visibility interact with the system prompt's capability catalog?

## Summary

Alan should unify its future skill system around three layers:

- `PortableSkill` for standards-compatible public skills
- `CapabilityPackage` for Alan-native hosting and distribution
- `PackageMount` for explicit capability exposure by agent roots

The key compatibility rule is:

- any standards-compatible public skill directory is automatically a valid single-skill package

The key unification rule is:

- today's system skills become built-in packages mounted by the default global base agent root

That gives Alan one coherent capability model that is both:

- aligned with the new agent-runtime design
- open to public skill ecosystems
