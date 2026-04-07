# Skill System Cutover Issues

> Owner doc: `docs/spec/skill_system_contract.md`
>
> This issue tree tracks the one-shot breaking cutover from package mount modes
> to the new skill exposure model:
>
> - `enabled`
> - `allow_implicit_invocation`

## Issue 1: Core Data Model And Config Cutover

Scope:

- remove `PackageMountMode`
- remove `PackageMount`
- replace `package_mounts` with `skill_overrides`
- resolve exposure per skill instead of per package
- add `enabled` and `allow_implicit_invocation` to resolved skill metadata
- parse `runtime.allow_implicit_invocation` from Alan sidecars
- parse `policy.allow_implicit_invocation` from tolerated compatibility
  metadata

Primary files:

- `crates/runtime/src/skills/types.rs`
- `crates/runtime/src/config.rs`
- `crates/runtime/src/agent_definition.rs`
- `crates/runtime/src/skills/capability_view.rs`
- `crates/runtime/src/skills/loader.rs`
- `crates/runtime/src/skills/registry.rs`
- `crates/runtime/src/skills/mod.rs`

Acceptance:

- runtime has no mount-mode type
- built-ins are discovered without always-active defaults
- overlays merge skill overrides field-by-field
- disabled skills stay resolved in catalog tooling but are not runtime-usable

## Issue 2: Prompt Assembly And Delegated Invocation Cutover

Scope:

- remove always-active activation
- remove keyword / pattern activation
- keep only direct skill-id force-selection for active-skill injection
- make the prompt catalog the implicit-discovery surface
- render inline skill catalog entries with canonical `SKILL.md` paths
- render delegated skill catalog entries with direct
  `invoke_delegated_skill` guidance
- remove mount-mode runtime context from active-skill prompt sections

Primary files:

- `crates/runtime/src/runtime/prompt_cache.rs`
- `crates/runtime/src/skills/injector.rs`
- `crates/runtime/src/runtime/virtual_tools.rs`
- `crates/runtime/prompts/runtime_base.md`

Acceptance:

- inline implicit skills are catalog-listed but not auto-injected
- delegated implicit skills are usable without prior active-skill injection
- explicit mention still renders active-skill runtime context
- no runtime path still depends on keyword/pattern or always-active activation

## Issue 3: Operator Surfaces And Persistence Cutover

Scope:

- expose `enabled` and `allow_implicit_invocation` in catalog snapshots
- remove package mount mode from CLI and daemon responses
- change daemon write route from `/api/v1/skills/mount_overrides` to
  `/api/v1/skills/overrides`
- persist `skill_overrides` in writable `agent.toml`

Primary files:

- `crates/alan/src/skill_catalog.rs`
- `crates/alan/src/cli/skills.rs`
- `crates/alan/src/daemon/routes.rs`
- `crates/alan/src/daemon/state.rs`

Acceptance:

- catalog package snapshots do not expose mount modes
- catalog skill snapshots expose `enabled` and
  `allow_implicit_invocation`
- CLI output no longer prints mode labels
- daemon writes and reads use skill ids, not package ids

## Issue 4: Test Surface Rewrite

Scope:

- delete mount-mode assertions
- rewrite config, capability view, registry, prompt cache, CLI, and daemon
  tests to the new matrix
- update fixtures and snapshots for non-always-active built-ins

Primary files:

- `crates/runtime/src/skills/types.rs`
- `crates/runtime/src/skills/capability_view.rs`
- `crates/runtime/src/skills/registry.rs`
- `crates/runtime/src/runtime/prompt_cache.rs`
- `crates/runtime/src/config.rs`
- `crates/runtime/src/agent_definition.rs`
- `crates/alan/src/skill_catalog.rs`
- `crates/alan/tests/skills_cli_integration_test.rs`

Acceptance:

- tests cover the matrix in `docs/spec/skill_system_contract.md`
- no test depends on `always_active`, `discoverable`, `explicit_only`, or
  `internal`
- no prompt-cache test expects keyword/pattern auto-activation

## Suggested Execution Order

1. Issue 1
2. Issue 2
3. Issue 3
4. Issue 4

This order matches the dependency chain: data model first, then runtime
behavior, then operator surfaces, then validation rewrite.
