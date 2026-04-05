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
- **First-party skill package**: a skill package shipped by Alan from a built-in
  source. It follows the same package contract as externally discovered
  packages.
- **Portable skill**: the shared public contract centered on `SKILL.md`.
- **Alan sidecar**: optional `skill.yaml` / `package.yaml` metadata that extends
  runtime behavior without changing `SKILL.md`.
- **Host tool**: a runtime capability registered through Alan's tool system and
  exposed uniformly to the model.
- **Package-local helper**: deterministic executable or support logic shipped
  inside a skill package, usually under `scripts/`.
- **Reusable skill tooling**: authoring/eval tooling that may be reused across
  multiple skill packages without becoming a runtime tool by default.
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

- `evals/evals.json` and input fixtures under `evals/files/`
- `agents/*.md`
- validator / benchmark scripts
- grader / analyzer prompts
- benchmark/review viewers and other package-local review assets

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
├── evals/              # optional authoring/eval manifests and fixtures
├── eval-viewer/        # optional authoring/eval review assets
└── agents/             # optional Alan-native child-agent exports
```

Rules:

1. A directory-backed package currently exports **exactly one portable skill**:
   the `SKILL.md` in the package root.
2. `scripts/`, `references/`, and `assets/` are the stable bundled resource
   directories.
3. `evals/` and `eval-viewer/` are optional authoring/evaluation surfaces for
   explicit tooling. Runtime discovery must ignore them by default.
4. `agents/` is an Alan-native extension directory for package-local child-agent
   exports and may also contain non-runtime authoring assets.
5. Unknown additional files or directories must be ignored by runtime
   discovery.
6. Multi-skill filesystem packages are **not** part of the stable public
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

### Tolerated But Ignored Compatibility Input

The following fields may appear in public skill assets, but Alan does not
preserve or consume them as part of the resolved runtime contract:

- `capabilities.optional_tools`
- `capabilities.domains`
- `capabilities.triggers.semantic`

They are tolerated as compatibility input, not as stable behavior.

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

### Tolerated But Ignored

- `runtime.ui`

Alan may tolerate this input, but it is not part of the stable contract and is
not preserved in resolved runtime metadata.

## Discovery Contract

Alan discovers skill packages from:

- built-in first-party packages
- `AgentRoot` `skills/` directories
- public `.agents/skills/` directories

Discovery is separate from runtime exposure. After discovery, packages enter the
resolved capability view and are then filtered by mount mode and availability.
Built-in first-party packages are not a separate package kind; `builtin` is a
discovery source and precedence tier, not a different runtime contract.

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

- Built-in first-party packages are discovered from the default global base
  agent root.
- The shipped base agent definition may mount some first-party packages
  `always_active` by default, but built-in source alone does not imply a fixed
  mount mode.
- Later overlays may still override built-in package mounts.

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

## Tool And Helper Contract

Alan separates **host tools**, **package-local helpers**, and **reusable skill
tooling**.

Rules:

1. Runtime tools are host capabilities registered through Alan's tool system and
   exposed uniformly to the model.
2. Skill packages do **not** create new runtime tool definitions merely by
   shipping files in the package tree.
3. Package-local deterministic helpers belong under `scripts/` and are invoked
   through host tools such as `bash`, or through explicit first-party authoring
   flows such as `alan skills eval`.
4. If a skill depends on an external executable that is not shipped inside the
   package, authors should declare it through `capabilities.required_tools` or
   `compatibility.dependencies` with dependency kind `tool`.
5. Whether a helper is implemented as shell, Python, Rust, or another compiled
   binary is an implementation detail. The stable contract sees either a
   packaged helper under `scripts/` or an external tool dependency.
6. Reusable skill tooling may be shared across multiple skill packages, but it
   remains operator-side tooling unless Alan explicitly promotes it into the
   runtime tool surface.
7. Reusable skill tooling should be callable from explicit authoring/eval
   workflows or package scripts without requiring every skill to vend its own
   top-level host CLI.
8. The `alan` CLI is a host/operator surface. Skill-private helper behavior
   should not be elevated into dedicated top-level `alan` subcommands unless it
   becomes an intentionally cross-skill workflow.

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
- `alan skills init`
- `alan skills validate`
- `alan skills eval`
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
4. `alan skills eval` is the explicit entry point for package-local eval hooks,
   manifests, and richer benchmark/review workflows when present

## First-Party Package Distribution

Alan should be able to ship first-party packages as **ordinary directory-backed
skill packages**, not merely as root-level `SKILL.md` strings.

Rules:

1. Built-in distribution is a packaging detail, not a different contract. A
   first-party package may carry the same `scripts/`, `references/`, `assets/`,
   `evals/`, `eval-viewer/`, `agents/`, and compatibility metadata as an
   external package.
2. The packaged asset view consumed by discovery, catalog surfaces, prompt
   assembly, and authoring tooling must preserve the same relative-path behavior
   as an external filesystem package.
3. First-party package source does not imply `always_active`; mount policy stays
   separate.
4. First-party packages may include authoring/eval assets, but those assets
   remain explicit tooling surfaces unless promoted into real runtime child-agent
   exports.

## First-Party Authoring Expectations

Alan's first-party skill authoring guidance should follow these rules:

1. Treat `description` as the trigger contract. It must say what the skill does
   and when to use it.
2. Keep `SKILL.md` lean. Move detailed reference material into `references/`.
3. Prefer deterministic helper scripts over repeatedly rewritten code.
4. Avoid clutter files such as `README.md`, `CHANGELOG.md`, and
   process-history notes inside skill packages.
5. First-party tooling should provide `init`, `validate`, and benchmark/eval
   flows over the same directory-backed skill package contract.
6. First-party Alan should be able to ship a full `skill-creator` package over
   that same contract, rather than relying only on ad hoc templates or
   documentation.
7. Rich eval loops may use package-local manifests, graders, analyzers,
   comparators, trigger-eval helpers, and review viewers, but they should stay
   explicit authoring surfaces.
8. Shared authoring/eval helpers that are reused across multiple skills should
   remain a tooling layer separate from Alan runtime tools and from skill-local
   package helpers by default.
9. Grader/analyzer/eval assets are valuable, but they should be treated as
   explicit authoring/evaluation surfaces or upgraded into real Alan child-agent
   exports, not silently loaded into runtime.

The authoring toolchain is not a second package system. `alan skills init`,
`alan skills validate`, and `alan skills eval` operate over ordinary skill
packages and their bundled assets.

## Explicit Non-Goals / Removed From The Stable Contract

These are intentionally outside the stable skill contract for now:

- `package.toml` manifests
- multi-skill filesystem packages
- semantic trigger activation
- `domains` and `optional_tools` as activation or availability semantics
- `viewers/` as a capability export or runtime contract
- `runtime.ui` as stable behavior
- nested delegated execution in V1

If Alan tolerates some of these as compatibility input, that must not be
mistaken for a stable behavior promise.
