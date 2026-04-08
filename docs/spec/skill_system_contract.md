# Skill System Contract

> Status: authoritative skill-system contract.
>
> This document defines the target cutover contract for Alan's skill system.
> It is a breaking-change spec. The old mount-mode model is removed rather than
> migrated in place.

## Goals

Alan's skill system must optimize for six things at the same time:

1. **Portable public alignment** with Codex- and Claude-style skill
   directories.
2. **Zero-conversion installation** for public skills under `.agents/skills/`.
3. **Codex/Claude-style progressive disclosure** where `name` and
   `description` decide selection and `SKILL.md` loads only after selection.
4. **Description-driven selection** with no structured trigger schema.
5. **Small prompt footprint** through progressive disclosure and on-demand file
   reads.
6. **Alan-native delegated execution** without polluting the public `SKILL.md`
   portability contract.

## Stable Vocabulary

- **Skill package**: one directory-backed portable skill plus optional
  Alan-native sidecars, resources, and package-local launch targets.
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
- **Package-local launch target**: an Alan-native export under `agents/<name>/`
  that may be launched for delegated execution.
- **Implicit invocation**: a skill is listed in the prompt catalog so the model
  may decide to use it on demand.
- **Host-level force-select**: a host/runtime UX such as direct skill-name
  mention or `$skill-id` that asks the host to activate a skill directly. This
  is not portable skill metadata.
- **Active skill**: a skill force-selected for the current turn and
  rendered as active runtime context.
- **Parent runtime**: the runtime currently handling the user turn and, when
  supported, exposing `invoke_delegated_skill`.
- **Launch-root runtime**: a runtime started from a package-local launch target.
  Launch-root runtimes intentionally keep nested delegated execution disabled
  in V1.
- **Delegated skill**: a skill that resolves to a package-local launch target
  instead of parent-runtime inline instructions.

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
  policy hints such as `allow_implicit_invocation`

This metadata is not part of the core `SKILL.md` portability contract. Unknown
fields must remain fail-open. `SKILL.md` remains the canonical selection and
instruction contract; Alan sidecars remain the canonical Alan-native extension
surface.

### Tier 3: Authoring / Eval Companion Assets

Alan **should** preserve and ignore-by-default auxiliary authoring assets such
as:

- `evals/evals.json` and input fixtures under `evals/files/`
- `agents/*.md`
- validator / benchmark scripts
- grader / analyzer prompts
- benchmark / review viewers and other package-local review assets

These are not part of the default runtime activation contract. They are
authoring and evaluation surfaces. First-party Alan tooling may consume them
explicitly, but runtime discovery must not require them.

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
└── agents/             # optional Alan-native package-local launch targets
```

Rules:

1. A directory-backed package currently exports **exactly one portable skill**:
   the `SKILL.md` in the package root.
2. `scripts/`, `references/`, and `assets/` are the stable bundled resource
   directories.
3. `evals/` and `eval-viewer/` are optional authoring/evaluation surfaces for
   explicit tooling. Runtime discovery must ignore them by default.
4. `agents/` is an Alan-native extension directory for package-local launch
   targets and may also contain non-runtime authoring assets.
5. Unknown additional files or directories must be ignored by runtime
   discovery.
6. The runtime `skill_id` is a normalized lower-case hyphenated slug derived
   from the package directory name; separator variants such as `.`, `_`, and
   whitespace canonicalize to `-`.
7. Multi-skill filesystem packages are **not** part of the stable public
   contract.

## `SKILL.md` Contract

### Required Frontmatter

- `name`
- `description`

These are the portable selection contract and must remain sufficient for basic
public skill interoperability.

### Stable Optional Frontmatter

- `metadata.short-description`
- `metadata.tags`
- `capabilities.required_tools`
- `capabilities.disclosure.level2`
- `capabilities.disclosure.level3.references`
- `capabilities.disclosure.level3.scripts`
- `capabilities.disclosure.level3.assets`
- `compatibility.min_version`
- `compatibility.dependencies`
- `compatibility.requirements`

### Stable Semantics

- `name` and `description` are the only portable skill-authored fields that
  determine when a skill should be selected.
- `compatibility.min_version` is a hard availability gate.
- `compatibility.dependencies` is a typed availability gate. Stable dependency
  kinds are `env_var`, `tool`, and `runtime_capability`.
- `compatibility.requirements` is advisory remediation text only. It is not a
  typed availability gate.

## Alan Sidecar Contract

Alan-native sidecars extend runtime behavior without changing the public
`SKILL.md` contract.

### `skill.yaml`

Stable Alan-native keys:

- `runtime.execution.mode = inline | delegate`
- `runtime.execution.target`
- `runtime.allow_implicit_invocation`
- `runtime.permission_hints`

### `package.yaml`

`package.yaml` may provide `skill_defaults.runtime` with the same stable keys as
`skill.yaml`. Package defaults apply before the skill-local sidecar.

### `agents/openai.yaml`

Alan may also consume:

- `policy.allow_implicit_invocation`

Precedence for implicit-invocation defaults is:

1. `skill.yaml` `runtime.allow_implicit_invocation`
2. `package.yaml` `skill_defaults.runtime.allow_implicit_invocation`
3. `agents/openai.yaml` `policy.allow_implicit_invocation`
4. default `true`

### Tolerated But Ignored

- `runtime.ui`

Alan may tolerate this input, but it is not part of the stable contract and is
not preserved in resolved runtime metadata.

## Discovery Contract

Alan discovers skill packages from:

- built-in first-party packages
- `AgentRoot` `skills/` directories
- public `.agents/skills/` directories

Discovery is separate from runtime exposure. Every discovered package enters the
resolved capability view. Packages remain visible to catalog tooling even when
their exported skills are disabled or unavailable.

Built-in first-party packages are not a separate package kind; `builtin` is a
discovery source and precedence tier, not a different runtime contract.

## Exposure Contract

Exposure is resolved **per skill**, not per package.

Stable runtime exposure fields are:

- `enabled`
- `allow_implicit_invocation`

Semantics:

- `enabled = false`: the skill is disabled for this runtime. It must not appear
  in the prompt catalog and must not activate through explicit mention.
- `enabled = true && allow_implicit_invocation = true`: the skill may appear in
  the prompt catalog when it is also available.
- `enabled = true && allow_implicit_invocation = false`: the skill is hidden
  from the prompt catalog but explicit activation still works when the skill is
  available.

Rules:

1. `enabled` defaults to `true`.
2. `allow_implicit_invocation` defaults according to sidecar / compatibility
   metadata, then falls back to `true`.
3. Package identity stays relevant for resources, package-local launch targets, and
   provenance, but package-level mount policy is removed from the stable
   runtime contract.
4. Built-in source does not imply `enabled` or implicit listing by itself.

## Override Contract

Operator overrides are resolved through `agent.toml`:

```toml
[[skill_overrides]]
skill = "repo-review"
enabled = false

[[skill_overrides]]
skill = "deployment"
allow_implicit_invocation = false
```

Rules:

1. Overrides are keyed by runtime `skill` id.
2. `enabled` and `allow_implicit_invocation` are independent override fields.
3. Later `AgentRoot` overlays override earlier values field-by-field for the
   same skill without discarding unrelated overrides for other skills.
4. Package-level overrides are not part of the stable contract.

## Selection Contract

Portable skill selection is intentionally narrow.

### Portable Selection Rules

1. `name` and `description` are the only skill-authored fields that determine
   whether a portable skill should be selected.
2. `SKILL.md` body content loads only after the host or model has selected the
   skill.
3. There is no structured trigger schema, alias list, keyword list, regex
   list, semantic trigger list, or always-active contract in the stable skill
   format.

### Host-Level Force-Select

Hosts may expose a force-select control such as direct skill-name mention or
`$skill-id`.

Rules:

1. Only `enabled` skills may be force-selected.
2. Force-select is keyed by the runtime `skill` id only; portable skills do not
   declare extra aliases.
3. Force-select does not depend on `allow_implicit_invocation`.
4. Unavailable skills stay unavailable even when force-selected; runtime must
   surface the reason instead of silently injecting them.
5. Disabled skills behave like not-found skills at runtime.

## Prompt Catalog Contract

The system prompt includes a skills catalog built from skills that are:

- `enabled = true`
- `allow_implicit_invocation = true`
- available in the current runtime

This catalog is now part of the behavioral contract. It is how Alan tells the
model which skills it may choose on demand.

Rules:

1. The catalog must include runtime `skill_id`, portable `name`, portable
   `description`, and the canonical `SKILL.md` path for inline skills.
2. The catalog must tell the model to open `SKILL.md` only when the task
   requires that skill.
3. The catalog must make `name` and `description` the portable selection
   surface.
4. The catalog must use progressive disclosure language: read the skill file
   first, then load only referenced resources as needed.
5. Inline implicit skills are **not** injected into the prompt by default.
6. Delegated implicit skills must include enough metadata for direct tool use:
   `skill_id`, delegated `target`, and the instruction to call
   `invoke_delegated_skill`.
7. Core behavior that must always be present belongs in the base prompt or tool
   descriptions, not in always-active skills.

## Active Skill Rendering Contract

Force-selected skills may still render an active-skill section in the prompt.
That section is a runtime convenience surface, not the primary discovery
mechanism for implicit usage.

Rules:

1. Inline force-selected skills render runtime context plus the disclosed instruction
   body and referenced resources.
2. Delegated force-selected skills render runtime context plus a delegated-capability
   stub.
3. Active-skill runtime context must expose `enabled`,
   `allow_implicit_invocation`, canonical path metadata, availability, and
   execution state.
4. Active-skill runtime context must not mention removed mount-mode concepts.

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
- unknown compatibility hints from tolerated metadata

`capabilities.required_tools` is canonicalized into the same dependency gate as
`compatibility.dependencies`.

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
5. Reusable skill tooling may be shared across multiple skill packages, but it
   remains operator-side tooling unless Alan explicitly promotes it into the
   runtime tool surface.

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
6. Authoring should keep references shallow.

## Execution Contract

Each discovered skill resolves to exactly one of:

- `inline`
- `delegate(target=package-launch-target)`

### Default Inference

Package-local inference is deterministic:

1. no launch targets -> `inline`
2. same-name skill and launch-target export -> `delegate`
3. exactly one skill and one launch-target export -> `delegate`
4. otherwise -> unresolved -> unavailable

Alan must not guess across ambiguous package shapes.

### Delegated Execution

Delegated execution is an Alan-native runtime contract:

1. parent runtimes expose `invoke_delegated_skill`
2. delegated implicit skills may be invoked directly from the catalog without a
   prior active-skill injection step
3. delegated launch uses a package-local `SpawnTarget` and a fresh
   launch-root runtime
4. parent runtime tape records a bounded delegated result rather than replaying
   the launch-root transcript
5. launch-root rollout remains separately inspectable out of band

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

## Launch-Target Contract

Alan-native package-local launch targets live under:

```text
skill-name/
└── agents/
    └── reviewer/
        ├── agent.toml
        ├── persona/
        └── policy.yaml
```

Rules:

1. entries under `agents/` are package-local launch targets
2. exported roots must remain inside the package tree after canonicalization
3. symlinks that escape the package tree must be ignored
4. compatibility assets such as `agents/grader.md` are **not** runtime
   launch targets by themselves

## Management Contract

Alan's local-first management surface includes:

- `alan skills list`
- `alan skills packages`
- `alan skills init`
- `alan skills validate`
- `alan skills eval`
- `GET /api/v1/skills/catalog`
- `GET /api/v1/skills/changed?after=<cursor>`
- `POST /api/v1/skills/overrides`

Rules:

1. daemon skill APIs use the default workspace or a registered workspace alias /
   short id, not arbitrary filesystem paths
2. override writes persist through the highest-precedence writable `AgentRoot`
3. override writes reject unknown `skill_id` values; callers must use a runtime
   skill id from the current catalog
4. change detection is cursor-based so clients do not need full catalog reloads
   on every poll
5. catalog and daemon responses expose skill-level `enabled` and
   `allow_implicit_invocation`
6. package snapshots do not expose mount modes because package-level exposure is
   not part of the stable contract

## First-Party Package Distribution

Alan should ship first-party packages as **ordinary directory-backed skill
packages**, not as privileged always-active instruction blobs.

Rules:

1. Built-in distribution is a packaging detail, not a different contract.
2. First-party packages may carry the same `scripts/`, `references/`, `assets`,
   `evals/`, `eval-viewer/`, `agents/`, and compatibility metadata as an
   external package.
3. First-party package source does not imply implicit listing or explicit
   enablement overrides.
4. Any behavior that Alan needs unconditionally must live in the base prompt,
   tool descriptions, or dedicated runtime policy rather than in always-active
   skills.

## Breaking Changes And One-Shot Cutover

This contract intentionally removes the previous model rather than preserving
legacy behavior.

Removed from the stable runtime contract:

- `PackageMount`
- `PackageMountMode`
- `package_mounts`
- `always_active`
- `discoverable`
- `explicit_only`
- `internal`
- structured trigger metadata in `SKILL.md`
- keyword / regex / negative-keyword activation
- always-active built-in skills
- package-level runtime exposure policy

Required cutover behavior:

1. Config uses `skill_overrides`, not `package_mounts`.
2. Runtime prompt assembly only force-selects active skills from host-level
   direct skill references; portable skills do not declare extra trigger
   metadata.
3. The system prompt catalog becomes the only implicit-discovery surface.
4. CLI, daemon, and catalog surfaces expose `enabled` and
   `allow_implicit_invocation`, not mount modes.
5. Tests asserting mount-mode behavior must be deleted or rewritten.
6. No legacy compatibility shim is required by this contract.

## Validation And Test Matrix

The cutover is not complete until the old mount-mode test surface is replaced by
the following matrix.

### Core Resolution

- skill override merge resolves `enabled` and `allow_implicit_invocation`
  independently across overlays
- disabled skills are absent from explicit and implicit runtime surfaces
- `allow_implicit_invocation = false` skills are catalog-hidden but still
  force-selectable by direct `skill_id`

### Prompt Assembly

- inline implicit skills appear in the catalog with canonical `SKILL.md` paths
  and are not auto-injected
- delegated implicit skills appear in the catalog with `skill_id`, `target`,
  and direct `invoke_delegated_skill` guidance
- direct `skill_id` mention still renders active skill context for inline skills
- direct `skill_id` mention still renders delegated stubs for delegated skills
- no skill-authored alias / keyword / pattern / always-active activation remains

### Availability

- disabled skills render as not found
- enabled but unavailable skills render unavailable diagnostics on direct
  `skill_id` mention
- unavailable skills do not appear in the implicit catalog

### CLI / Daemon / Catalog

- catalog snapshot exposes `enabled` and `allow_implicit_invocation`
- package snapshots no longer expose mount modes
- daemon override writes target `skill_overrides`
- CLI output no longer prints mount-mode labels

### Documentation / Fixtures

- config examples use `skill_overrides`
- architecture and AGENTS summaries do not mention mount modes or always-active
  activation
- built-in fixtures and snapshots assume no always-active defaults

## Explicit Non-Goals / Removed From The Stable Contract

These are intentionally outside the stable skill contract for now:

- `package.toml` manifests
- multi-skill filesystem packages
- structured trigger metadata
- runtime mount policies
- `viewers/` as a capability export or runtime contract
- `runtime.ui` as stable behavior
- nested delegated execution in V1
