# Skill System Contract

> Status: authoritative skill-system contract.
>
> This document defines the stable package, activation, execution, and
> management model that future Alan skill work should target.
> [`../skills_and_tools.md`](../skills_and_tools.md) is the current
> implementation guide. The older skill plan documents under `plans/` are
> historical rollout rationale, not the source of truth for shipped behavior.

## Goals

Alan's skill system must optimize for five things at the same time:

1. **Portable public compatibility** with Codex- and Claude-style skill
   directories.
2. **Zero-conversion installation** for public skills under `.agents/skills/`.
3. **Explicit Alan-native extensions** for mounts, delegated execution, and
   child-agent exports without breaking portable `SKILL.md`.
4. **Deterministic activation** and **bounded delegated execution**.
5. **Small prompt footprint** through progressive disclosure and narrow result
   boundaries.

## Stable Vocabulary

- **Skill package**: one directory-backed portable skill plus optional
  Alan-native sidecars, resources, and child-agent exports.
- **Portable skill**: the shared public contract centered on `SKILL.md`.
- **Alan sidecar**: optional `skill.yaml` / `package.yaml` metadata that extends
  runtime behavior without changing `SKILL.md`.
- **Mount**: how a discovered package is exposed to the current runtime.
- **Active skill**: a selected skill with resolved availability and execution
  state for the current turn.
- **Delegated skill**: a skill that resolves to a package-local child-agent
  executor instead of parent-side inline instructions.

## Compatibility Profile

### Tier 1: Portable Runtime Compatibility

Alan **must** discover and run public skill directories shaped like:

```text
skill-name/
├── SKILL.md
├── scripts/
├── references/
└── assets/
```

This package shape must work without Alan-specific manifests when installed
under:

- `~/.agents/skills/`
- `<workspace>/.agents/skills/`
- Alan `AgentRoot` `skills/` directories

Unknown extra files must be ignored rather than treated as fatal.

### Tier 2: Compatibility Metadata

Alan **should** tolerate and consume public compatibility metadata when
present, especially:

- `agents/openai.yaml` from Codex-style skills for UI-facing metadata and
  dependency hints

This metadata is not part of the core `SKILL.md` portability contract. Unknown
fields must remain fail-open. `SKILL.md` remains the canonical trigger
contract; Alan sidecars remain the canonical Alan-native extension surface.
Compatibility metadata augments catalog/UI surfaces rather than replacing those
contracts.

### Tier 3: Authoring / Eval Companion Assets

Alan **should** preserve and ignore-by-default auxiliary authoring assets such
as:

- `agents/*.md`
- validator / benchmark scripts
- grader / analyzer prompts

These are not part of the default runtime activation contract. They are
authoring and evaluation surfaces. First-party Alan tooling may consume them
explicitly, but runtime discovery must not require them.

This keeps Alan compatible with Claude/Codex authoring conventions without
mistaking every authoring artifact for a runtime capability.

## Stable Package Layout

The stable directory-backed skill package contract is:

```text
skill-name/
├── SKILL.md
├── skill.yaml          # optional Alan-native skill metadata
├── package.yaml        # optional Alan-native package defaults
├── scripts/
├── references/
├── assets/
└── agents/             # optional Alan-native child-agent exports
```

Rules:

1. A directory-backed package currently exports **exactly one portable skill**:
   the `SKILL.md` in the package root.
2. `scripts/`, `references/`, and `assets/` are the stable bundled resource
   directories.
3. `agents/` is an Alan-native extension directory for package-local child-agent
   exports.
4. Unknown additional files or directories must be ignored by runtime
   discovery.
5. Multi-skill filesystem packages are **not** part of the stable public
   contract. If multiple skills are needed, author multiple sibling packages.

This intentionally removes an over-designed abstraction that is broader than the
current implementation and broader than the public Codex/Claude baseline.

## `SKILL.md` Contract

### Required Frontmatter

- `name`
- `description`

These are the portable trigger contract and must remain sufficient for basic
public skill interoperability.

### Stable Optional Frontmatter

- `metadata.short-description`
- `metadata.tags`
- `capabilities.required_tools`
- `capabilities.triggers.explicit`
- `capabilities.triggers.keywords`
- `capabilities.triggers.patterns`
- `capabilities.triggers.negative_keywords`
- `capabilities.disclosure.level2`
- `capabilities.disclosure.level3.references`
- `capabilities.disclosure.level3.scripts`
- `capabilities.disclosure.level3.assets`
- `compatibility.min_version`
- `compatibility.dependencies`
- `compatibility.requirements`

### Stable Semantics

- `compatibility.min_version` is a hard availability gate.
- `compatibility.dependencies` is a typed availability gate. Stable dependency
  kinds are `env_var`, `tool`, and `runtime_capability`.
- `compatibility.requirements` is advisory remediation text only. It is not a
  typed availability gate.

### Parsed But Not Stable Contract

The following fields may be tolerated for forward compatibility, but they are
not part of Alan's stable skill contract and must not be treated as required
authoring surface:

- `capabilities.optional_tools`
- `capabilities.domains`
- `capabilities.triggers.semantic`

If Alan continues parsing them, that is compatibility tolerance rather than a
stable behavior guarantee.

## Alan Sidecar Contract

Alan-native sidecars extend runtime behavior without changing the public
`SKILL.md` contract.

### `skill.yaml`

Stable Alan-native keys:

- `runtime.execution.mode = inline | delegate`
- `runtime.execution.target`
- `runtime.permission_hints`

### `package.yaml`

`package.yaml` may provide `skill_defaults` with the same stable keys as
`skill.yaml`. Package defaults apply before the skill-local sidecar.

### Not Yet Stable

- `runtime.ui`

Alan may continue to parse this data, but it is not part of the stable contract
until a real consumer exists.

## Discovery Contract

Alan discovers skill packages from:

- built-in first-party packages
- `AgentRoot` `skills/` directories
- public `.agents/skills/` directories

Discovery is separate from runtime exposure. After discovery, packages enter the
resolved capability view and are then filtered by mount mode and availability.

## Mount Contract

Stable mount modes are:

- `always_active`
- `discoverable`
- `explicit_only`
- `internal`

Semantics:

- `always_active`: visible and selected every turn
- `discoverable`: visible and activatable
- `explicit_only`: not listed in the catalog, but activatable by explicit
  mention
- `internal`: not exposed through the current runtime skill surface

Built-in first-party packages are mounted `always_active` by default from the
default global base agent root. Later overlays may still override those mounts.

## Activation Contract

Skill activation must be deterministic.

### Activation Sources

1. `always_active` package mounts
2. Explicit `$skill-id` mentions
3. Explicit aliases declared in `triggers.explicit`
4. Deterministic keyword / regex pattern matches on discoverable skills

### Activation Rules

1. Explicit mention always wins when the skill is exposed and available.
2. `negative_keywords` suppress automatic keyword/pattern activation, but do not
   suppress explicit mention.
3. Only `discoverable` skills participate in automatic keyword/pattern
   activation.
4. The rendered skills catalog is informational only. It is **not** itself an
   activation source or a model-side classifier contract.
5. Semantic-trigger activation is outside the stable contract.

## Availability Contract

Hard availability gates are:

- `capabilities.required_tools`
- `compatibility.min_version`
- `compatibility.dependencies`
- resolved delegated execution state

If a skill resolves to delegated execution ambiguously, Alan must mark it
unavailable rather than silently guessing or falling back inline.

Advisory only:

- `compatibility.requirements`
- `agents/openai.yaml` dependency hints with unknown kinds

`capabilities.required_tools` is canonicalized into the same dependency gate as
`compatibility.dependencies`. Tolerated compatibility metadata such as
`agents/openai.yaml` may contribute typed dependency hints when Alan recognizes
the dependency kind. Unknown compatibility hints, including MCP-oriented hints
in the current Alan runtime, remain fail-open.

## Progressive Disclosure Contract

Alan uses three disclosure levels:

1. **Metadata**: `name`, `description`, `short-description`, tags
2. **Primary instruction body**: `SKILL.md` body or `disclosure.level2`
3. **Bundled resources**: `references/`, `scripts/`, `assets/`

Rules:

1. `SKILL.md` should stay concise and procedural.
2. Detailed schemas, examples, and domain reference material should move into
   `references/`.
3. Deterministic scripts should live in `scripts/`.
4. Templates and output resources should live in `assets/`.
5. Relative resource paths resolve against the canonical package resource root.
6. Authoring should keep references shallow: `SKILL.md` should point directly to
   the resources that matter rather than rely on deep reference chains.

This follows the same context-discipline emphasized by public Claude/Codex
skills.

## Execution Contract

Each discovered skill resolves to exactly one of:

- `inline`
- `delegate(target=package-child-agent)`

### Default Inference

Package-local inference is deterministic:

1. no child-agent exports -> `inline`
2. same-name skill and child-agent export -> `delegate`
3. exactly one skill and one child-agent export -> `delegate`
4. otherwise -> unresolved -> unavailable

Alan must not guess across ambiguous package shapes.

### Delegated Execution

Delegated execution is an Alan-native runtime contract:

1. top-level runtimes expose `invoke_delegated_skill`
2. parent prompt receives a lightweight capability stub instead of the full
   `SKILL.md` body
3. delegated launch uses a package-local `SpawnTarget` and a fresh child
   runtime
4. default launch stays narrow: current default handles are `Workspace` and
   `ApprovalScope`
5. parent tape records a bounded delegated result rather than replaying the
   child transcript
6. child rollout remains separately inspectable out of band

Delegated execution must not implicitly inherit:

- parent tape
- active skills
- plan state
- memory handle
- nested delegated execution

### Runtime Capability Fallback

There is no third user-facing execution mode beyond `inline` and `delegate`.
However, when a runtime does not expose delegated invocation support, a
delegated skill may fall back to inline rendering for that runtime only. This is
a runtime capability fallback, not a stable author-facing execution mode.

## Child-Agent Export Contract

Alan-native child-agent exports live under:

```text
skill-name/
└── agents/
    └── reviewer/
        ├── agent.toml
        ├── persona/
        └── policy.yaml
```

Rules:

1. child-agent exports are package-local launch targets
2. exported roots must remain inside the package tree after canonicalization
3. symlinks that escape the package tree must be ignored
4. compatibility assets such as `agents/grader.md` are **not** runtime
   child-agent exports by themselves

This is the key distinction between Alan's runtime child-agent model and the
authoring/eval assets commonly found in Claude/Codex skills.

## Management Contract

Alan's local-first management surface includes:

- `alan skills list`
- `alan skills packages`
- `GET /api/v1/skills/catalog`
- `GET /api/v1/skills/changed?after=<cursor>`
- `POST /api/v1/skills/mount_overrides`

Rules:

1. daemon skill APIs use the default workspace or a registered workspace alias /
   short id, not arbitrary filesystem paths
2. mount-override writes persist through the highest-precedence writable
   `AgentRoot`
3. change detection is cursor-based so clients do not need full catalog reloads
   on every poll

## First-Party Authoring Expectations

Alan's first-party skill authoring guidance should follow these rules:

1. Treat `description` as the trigger contract. It must say what the skill does
   and when to use it.
2. Keep `SKILL.md` lean. Move detailed reference material into `references/`.
3. Prefer deterministic helper scripts over repeatedly rewritten code.
4. Avoid clutter files such as `README.md`, `CHANGELOG.md`, and
   process-history notes inside skill packages.
5. First-party tooling should eventually provide `init`, `validate`, and
   benchmark/eval flows.
6. Grader/analyzer/eval assets are valuable, but they should be treated as
   explicit authoring/evaluation surfaces or upgraded into real Alan child-agent
   exports, not silently loaded into runtime.

## Explicit Non-Goals / Removed From The Stable Contract

These are intentionally outside the stable skill contract for now:

- `package.toml` manifests
- multi-skill filesystem packages
- semantic trigger activation
- `domains` and `optional_tools` as activation or availability semantics
- `viewers/` as a runtime contract
- `runtime.ui` as stable behavior
- nested delegated execution in V1

If Alan keeps parsing some of these for compatibility, that must not be
mistaken for a stable behavior promise.
