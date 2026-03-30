# Alan Delegated Skill Execution Plan (2026-03-24)

## Context

Alan's current skill system now has a much cleaner definition-layer model than before:

- `CapabilityPackage`
- `PackageMount`
- `ResolvedCapabilityView`
- package exports for both skills and child-agent roots

Alan's runtime direction is also already clear:

- `AgentRoot`
- `AgentInstance`
- `SpawnSpec`
- child startup as `exec`, not prompt-level cloning

What Alan still does not have is a clean way to connect these two layers for capabilities whose
best execution model is not inline prompt injection.

Today, the skill path is still fundamentally inline:

- a skill is selected
- its instructions are injected into the parent agent's prompt
- the parent agent performs the work itself

That works for lightweight instruction extensions, but it is the wrong shape for capabilities that:

- consume a lot of intermediate context
- perform long inspection or evaluation work
- should run under a tighter or more specialized child configuration
- only need to return a concise result to the parent

Examples include:

- review and grading flows
- repository analysis
- long extraction and summarization passes
- evaluation or scoring tasks
- capability-specific analyzers that should not pollute the parent tape

Alan therefore needs a first-class concept for skills whose normal execution path is child-agent
delegation rather than inline expansion.

## Problem Statement

Alan's current skill model conflates two different questions:

1. how a capability is discovered and selected
2. how that capability is executed once selected

This creates three limitations.

### 1. Inline skills bloat parent context

When a skill is executed inline, the parent agent absorbs:

- the skill instructions
- the intermediate work
- the tool outputs
- the final result

For some capabilities, this is unnecessary and expensive.

### 2. Package child-agent exports are passive

Alan's package model already has room for child-agent exports, but those exports are not yet tied
to skill execution semantics.

That means Alan can discover:

- user-facing skills
- internal child agents

but it still cannot say:

- "this skill should normally execute by launching that child agent"

### 3. There is no stable result boundary

Without a delegated-skill contract, Alan has no first-class way to say:

- run this capability in an isolated child instance
- keep the child's full tape out of the parent
- return only a bounded result object

That boundary is one of the main reasons delegated execution is useful at all.

## Goals

1. Introduce a first-class delegated skill model on top of `CapabilityPackage` and `SpawnSpec`.
2. Make delegated execution the default when package shape makes it obvious, without requiring
   heavy manual configuration.
3. Keep delegated-skill execution aligned with Alan's `exec` model:
   - fresh child instance
   - default non-inheritance
   - explicit bindings only
4. Return a bounded result to the parent instead of replaying the child's full transcript into
   parent context.
5. Keep portable public skills portable and avoid putting Alan-specific execution metadata into
   `SKILL.md`.
6. Preserve simple inline skills as the default model for capabilities that do not need child
   delegation.

## Non-Goals

1. Do not redesign the external portable skill format.
2. Do not require every skill package to define child agents.
3. Do not make delegated skills automatically inherit parent tape, plan state, memory, or active
   skills.
4. Do not introduce a general multi-agent mesh or peer-to-peer capability graph in this phase.
5. Do not make delegated execution recursive by default.
6. Do not expose fallback-heavy compatibility behavior as a primary user-facing model in the first
   version.

## Design Principles

1. Separate capability selection from capability execution.
2. Keep delegated execution strict and predictable.
3. Default to zero-config delegation only when package shape is unambiguous.
4. Put Alan-specific execution metadata in optional sidecar metadata, not in `SKILL.md`.
5. Treat delegated execution as a runtime contract, not as prompt magic.
6. Return results, not transcripts.
7. Preserve `exec`, not `fork`.

## Core Model

Alan should distinguish between two kinds of skill execution.

### 1. Inline Skill

An inline skill is executed by the parent agent.

Behavior:

- the skill becomes active in the parent capability view
- the parent receives its prompt-facing instructions
- the parent performs the work itself

This is the right model for:

- lightweight behavioral guidance
- local workflow instructions
- skills that do not justify another agent instance

### 2. Delegated Skill

A delegated skill is executed by a child agent instance launched by the runtime.

Behavior:

- the parent sees the skill as an invokable capability, not as a large prompt injection
- the runtime resolves a child-agent target for the skill
- invoking the skill launches a fresh child instance via `SpawnSpec`
- the parent receives only the delegated result

This is the right model for:

- high-context or long-running subtasks
- specialized review or evaluation work
- capabilities that should use a tighter agent definition or tool profile
- capabilities where the parent only needs the output, not the full process

## User-Facing Execution Model

The first version should expose only two user-facing execution modes:

- `inline`
- `delegate`

Alan should not expose `delegate_preferred` as a primary first-version concept.

### Why Not `delegate_preferred` in V1

`delegate_preferred` mixes two independent concerns:

- who should execute the work
- whether runtime may silently fall back to inline execution

That fallback weakens the most important promise of delegated skills:

- parent context stays small because the work runs elsewhere

If Alan silently falls back to inline execution, the parent may suddenly absorb:

- the skill body
- the intermediate steps
- the tool outputs

That makes behavior less predictable for both authors and users.

The cleaner first version is:

- `inline` means parent execution
- `delegate` means child execution

If Alan later needs a compatibility fallback mode, it can be introduced as an internal runtime
policy or a future extension, but it should not define the core model.

## Capability Selection Versus Execution

Delegated skills should still participate in normal capability selection:

- package mount resolution
- visibility rules
- trigger matching
- explicit mention
- availability checks

But selection alone should not mean "inject everything into the parent prompt."

Alan should explicitly split the flow:

1. select which skills are relevant and available
2. determine each selected skill's execution mode
3. expose inline skills as parent instructions
4. expose delegated skills as lightweight callable capabilities

This keeps the parent aware that the capability exists without forcing it to absorb the delegated
skill's whole execution model.

## Runtime Contract for Delegated Skills

The first delegated-skill runtime should be explicit and synchronous from the parent turn's point
of view.

### Parent-Side Exposure

When a delegated skill is selected, the parent should receive a lightweight capability stub rather
than the full `SKILL.md` body.

That stub should include at least:

- skill id
- description
- short usage guidance
- the fact that the skill is delegated
- the invocation path the parent should use

The parent should not receive the delegated skill's full prompt body by default.

### Invocation Path

Alan should introduce a runtime-native invocation path for delegated skills.

Illustrative shape:

```text
invoke_delegated_skill(
  skill = "repo-review",
  task = "Review the current diff for correctness and missing tests"
)
```

The first shipped implementation should be:

- a dedicated virtual tool such as `invoke_delegated_skill`

The deeper semantic rule is:

- delegated skill execution should be an explicit runtime action, not implicit prompt expansion

Alan can generalize this into a broader runtime-native capability invocation layer later if that
becomes useful.

### Child Launch Semantics

Invoking a delegated skill should:

- resolve its target child-agent export
- create a fresh child instance
- launch that instance through `SpawnSpec`
- bind only the minimal default handles

The child launch is `exec`-like, not `fork`-like.

The child should not inherit by default:

- parent tape
- parent active skills
- parent prompt state
- parent memory view
- parent plan state
- parent approval cache
- parent dynamic tools

### Default Handle Policy

The delegated child should receive the smallest useful default set.

Recommended defaults:

- `workspace`
- `result_channel`

Everything else should require later explicit design.

This keeps delegated skills honest:

- they solve the delegated task with a fresh instance
- they do not depend on hidden parent state

In particular, V1 should not bind `artifacts` by default.

### Result Handoff

Delegated execution should return a bounded result object rather than replaying the entire child
conversation into the parent tape.

The V1 default result shape should stay minimal:

```text
DelegatedSkillResult
  status
  summary
  structured_output
```

The parent should consume:

- the fact that the skill was invoked
- the delegated result

The parent should not automatically consume:

- the child's full transcript
- every child tool output
- the child's intermediate reasoning

If later debugging or inspection is needed, Alan can expose the child rollout separately, but that
should be explicit and out-of-band.

`artifact_refs` should not be part of the default V1 contract. If later versions add an explicit
`artifacts` handle, artifact references can be layered on as an optional extension rather than as a
baseline requirement.

## Delegated Skill Targets

The first version should keep targets simple and local.

### V1 Target Rule

A delegated skill should target a child-agent export from the same capability package.

This is the cleanest first contract because it is:

- package-local
- explicit
- hermetic
- aligned with package child-agent exports already present in the model

Alan should not require delegated skills to target arbitrary global named agents in the first
version.

That broader target model can be added later if it becomes necessary.

## Default Inference Rules

Alan should support zero-config delegation when the package shape is obvious.

### Rule 1: No Child-Agent Export Means Inline

If a package exports a skill and no child-agent roots, the effective execution mode is:

- `inline`

### Rule 2: Same-Name Skill and Child Agent Means Delegate

If a package exports:

- a skill `foo`
- a child-agent export `foo`

Alan should infer:

- skill `foo` executes as `delegate`
- target child agent is `foo`

This should be the recommended zero-config authoring convention for delegated skills.

### Rule 3: One Skill and One Child Agent Means Delegate

If a package exports exactly:

- one skill
- one child-agent export

Alan should infer delegation from the skill to that child agent, even if the names differ.

### Rule 4: Ambiguous Shapes Require Explicit Metadata

If a package exports multiple skills and/or multiple child-agent roots such that delegation is not
obvious, Alan should not guess.

In that case:

- the package must provide explicit sidecar metadata for that skill
- Alan should mark delegated execution for that skill unresolved until metadata is present
- the runtime should surface a validation warning and should not silently fall back to inline

This is stricter than defaulting to inline, but it keeps delegated semantics honest and avoids
silently expanding high-cost skills into the parent context by accident.

## Authoring Convention for Zero-Config Delegated Skills

The easiest delegated-skill package should look like this:

```text
my-skill/
  skills/
    my-skill/
      SKILL.md
  agents/
    my-skill/
      agent.toml
```

This should mean, by convention:

- `my-skill` is a delegated skill
- its executor is the same-name child-agent export
- no extra Alan-specific execution config is required

That convention keeps authoring simple and gives Alan a strong default.

## Sidecar Metadata Placement

Delegated-skill execution metadata should not live in `SKILL.md`.

This belongs in the optional Alan sidecar metadata introduced by the skill-system productization
work.

Illustrative shape:

```yaml
skills:
  skill-creator:
    execution:
      mode: delegate
      target: creator
```

This metadata is only needed when package shape is ambiguous or when the package wants to override
the default inference rules.

## Relationship Between `SKILL.md` and the Child Agent

For a delegated skill, the responsibilities should be split cleanly.

### `SKILL.md` Remains the Parent-Facing Capability Surface

It should continue to provide:

- name
- description
- trigger-facing guidance
- package-visible documentation

### The Child Agent Owns Execution

The target child-agent export should own:

- execution-specific persona
- tool profile and policy defaults
- any agent-specific workflow logic

This means a delegated skill is not "a normal inline skill that happens to spawn."

It is a capability entry point whose actual executor is a child agent definition.

That keeps the model aligned with `AgentRoot` and `SpawnSpec` instead of hiding execution inside
prompt text.

## Example: Simple Delegated Review Skill

```text
repo-review/
  skills/
    repo-review/
      SKILL.md
  agents/
    repo-review/
      agent.toml
      persona/
        ROLE.md
        TOOLS.md
  references/
  scripts/
```

Effective behavior:

- package exports user-facing skill `repo-review`
- package exports child agent `repo-review`
- Alan infers `repo-review` is delegated
- the parent sees a lightweight `repo-review` capability
- invoking it launches the `repo-review` child
- the child returns a concise review result

## Example: Rich Package With Internal Helpers

```text
skill-creator/
  skills/
    skill-creator/
      SKILL.md
  agents/
    creator/
      agent.toml
    grader/
      agent.toml
    analyzer/
      agent.toml
```

This shape is ambiguous.

Alan should not guess whether `skill-creator` should map to:

- `creator`
- `grader`
- `analyzer`

This package should provide explicit sidecar metadata such as:

```yaml
skills:
  skill-creator:
    execution:
      mode: delegate
      target: creator
```

`grader` and `analyzer` can remain internal package exports used by the delegated executor.

## Nested Delegation in V1

The first delegated-skill implementation should stop at one explicit boundary:

- parent agent invokes delegated skill
- delegated child executes task
- delegated child returns bounded result

Alan should not enable nested delegated-skill chains in V1.

That means:

- delegated executors should not themselves invoke other delegated skills by default
- package-internal child-agent orchestration beyond the delegated executor should wait until the
  basic result boundary and supervision model are proven

This keeps V1 simpler in three ways:

- one supervisor boundary per delegated invocation
- one result handoff contract
- no recursive delegation policy questions in the first release

## Relationship to Existing Plans

This design is layered on top of existing Alan plans, not separate from them.

### Depends on the Skill Package / Mount Work

Delegated skills assume the package model already exists:

- skills and child-agent exports come from `CapabilityPackage`
- agent roots expose packages through mounts
- runtime consumes a resolved capability view

### Depends on the Agent Root Runtime Work

Delegated skills also assume:

- child-agent startup is `exec`
- runtime has a stable child-agent launch primitive
- shared state is passed only through explicit `SpawnSpec` bindings

In other words:

- package work defines what can be exported
- runtime work defines how a delegated skill can execute
- this plan connects the two

## Desired End State

By the end of this work:

- Alan supports both inline skills and delegated skills as first-class execution modes
- package shape can infer delegated execution in the common one-skill/one-agent case
- delegated skills are invoked through a runtime-native capability path
- delegated execution launches a fresh child instance
- delegated results return as bounded outputs instead of full transcript replay
- parent context stays smaller for high-cost specialized capabilities

## Phase Plan

### PR1: Formalize Delegated Skill Terminology and Resolved Metadata

Goal: define the model before wiring runtime behavior.

Changes:

- document `inline` versus `delegate`
- add resolved execution mode and delegate target to capability metadata
- define default inference rules for package shape

Likely issue links:

- `#168`
- `#169`
- `#170`

### PR2: Introduce Parent-Side Delegated Skill Exposure

Goal: expose delegated skills to the parent as lightweight callable capabilities instead of full
instruction injection.

Changes:

- add delegated capability stubs to prompt assembly
- avoid injecting full delegated skill bodies into the parent prompt
- define the runtime-native invocation path

Likely issue links:

- `#169`
- `#172`

### PR3: Wire Delegated Skill Invocation to Child-Agent Launch

Goal: connect delegated skill invocation to package-export child-agent execution.

Changes:

- resolve delegated targets from package exports
- launch child agents via `SpawnSpec`
- enforce default non-inheritance
- bind `workspace` and `result_channel` by default

Likely issue links:

- `#108`
- `#175`

### PR4: Define Delegated Result Handoff

Goal: make the result boundary explicit and stable.

Changes:

- define `DelegatedSkillResult`
- record delegated invocation and result in the parent rollout
- keep child transcript out of parent context by default

Likely issue links:

- `#168`
- `#173`

### PR5: Harden Authoring and UX

Goal: make delegated skills easy to author and easy to understand.

Changes:

- document the zero-config same-name convention
- add `alan skills` inspection output for execution mode and delegate target
- validate ambiguous package shapes with clear diagnostics

Likely issue links:

- `#174`

## V1 Decisions

1. Ambiguous package shapes require explicit sidecar metadata. Alan should mark delegated execution
   as unresolved and surface a validation warning instead of silently defaulting to inline.
2. The first delegated invocation path should be a dedicated virtual tool such as
   `invoke_delegated_skill`, while leaving room for a broader capability invocation layer later.
3. The default V1 `DelegatedSkillResult` should include `status`, `summary`, and optional
   `structured_output`, but not `artifact_refs`.
4. The default V1 child-handle set should stay minimal: `workspace` plus `result_channel`. Do not
   bind `artifacts` by default.
5. Nested delegated execution should wait until the base delegated-skill model is proven. V1
   should assume one delegated boundary per invocation.

## Summary

Alan should add a first-class delegated skill model that sits between package discovery and child
agent execution.

The key simplifications are:

- only two user-facing execution modes in V1: `inline` and `delegate`
- zero-config delegation when package shape is unambiguous
- delegated skills execute through a runtime-native invocation path, not prompt expansion
- delegated execution launches a fresh child instance through `SpawnSpec`
- the parent receives a bounded result instead of the child's full working context

This gives Alan a much cleaner path to "skills that outsource work" without collapsing back into
prompt cloning or hidden inheritance.
