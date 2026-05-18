## Context

alan historically accumulated durable contract text in `docs/spec/`, active or
semi-active execution plans in `plans/`, and task-specific Superpowers plans
under `docs/superpowers/`. OpenSpec is now the desired contract system, but the
repository still presents `docs/spec/` as a parallel spec tree and several docs
still point readers there for normative behavior.

The cleanup has to preserve useful operational documentation while removing
duplicate sources of truth. Current implementation guides, maintainer runbooks,
harness fixtures, and live validation instructions are still useful outside
OpenSpec. Durable requirements, product/runtime contracts, and target behavior
must move into `openspec/specs/` or active `openspec/changes/`.

There is also active terminal work in the current worktree. This change should
avoid touching unrelated Apple runtime files except for documentation references
that are directly part of the spec-source cleanup.

## Goals / Non-Goals

**Goals:**

- Make `openspec/specs/` the only durable specification source.
- Make `openspec/changes/` the only home for in-flight spec/design/task work.
- Reclassify `docs/` content into guides, runbooks, validation instructions,
  executable fixtures, or short bridge pointers.
- Remove historical Superpowers and `plans/` files after any still-current
  decisions are captured in OpenSpec or guide docs.
- Add validation expectations so new docs do not recreate a parallel spec
  system.

**Non-Goals:**

- Do not rewrite runtime behavior or daemon APIs.
- Do not preserve historical execution plans as a repository archive; git
  history is enough once their decisions are captured.
- Do not bulk-delete current implementation guides, maintainer runbooks, live
  validation instructions, or harness scenario fixtures.
- Do not rewrite archived OpenSpec history except where active validation needs
  non-archived references to be corrected.

## Decisions

### Decision: Add one governance capability before migrating individual specs

`documentation-governance` owns the taxonomy and source-of-truth rules. It does
not try to own every runtime, provider, UI, or testing requirement. Those
requirements should move into their domain capabilities as the migration
proceeds.

Alternative considered: immediately create a large set of new runtime,
provider, governance, and UI capabilities mirroring every `docs/spec/*.md`
file. Rejected because it would create a large mechanical diff before the
repository has a single rule that prevents future drift.

### Decision: Keep non-normative docs outside OpenSpec

`docs/architecture.md`, `docs/skills_and_tools.md`, live harness guides, and
maintainer runbooks may stay in `docs/` when they describe current usage,
operator commands, troubleshooting, or implementation context. They must link
to OpenSpec for normative requirements rather than declaring their own target
contracts.

Alternative considered: move all Markdown under `docs/` into OpenSpec. Rejected
because OpenSpec is for behavior contracts and change artifacts, not every
operator guide or executable fixture README.

### Decision: Replace stale contract docs with bridge pages only when needed

Some old paths are still linked from README, AGENTS, archived changes, or user
habits. During migration, a deleted contract file may be replaced by a short
bridge that says it is not authoritative and points to the OpenSpec capability.
Bridge pages should be removed once active references are updated.

Alternative considered: leave full legacy docs in place with a status banner.
Rejected because long pages continue to look authoritative and become a second
source of truth.

### Decision: Clean historical plans instead of archiving them in docs

`plans/` and `docs/superpowers/` are execution history, not durable product
truth. If a plan still guides work, its open decisions should be moved into an
active OpenSpec change. If it is implemented or superseded, delete it.

Alternative considered: keep a permanent historical plans directory. Rejected
because it makes old implementation sequencing look current and duplicates git
history.

## Risks / Trade-offs

- [Risk] A useful current guide is deleted because it resembles a spec.
  Mitigation: classify files by purpose first; keep current guides and rewrite
  only their normative sections into OpenSpec.
- [Risk] A bridge page becomes a permanent duplicate.
  Mitigation: each bridge must name its OpenSpec replacement and have a cleanup
  task.
- [Risk] Migrating all `docs/spec/` files in one pass creates a noisy,
  unreviewable diff.
  Mitigation: establish governance and first cleanup first, then migrate
  domain groups in reviewable slices.
- [Risk] Active terminal work mixes with documentation cleanup.
  Mitigation: keep this change scoped to OpenSpec/docs files and avoid unrelated
  Apple runtime edits.

## Migration Plan

1. Add `documentation-governance` and validation expectations.
2. Update docs indexes and AGENTS guidance to make OpenSpec the only spec
   source.
3. Remove or absorb historical `docs/superpowers/` and `plans/` files.
4. Convert `docs/spec/` files in domain slices:
   - macOS shell/product UI docs,
   - runtime/kernel/execution/memory docs,
   - provider/connection docs,
   - governance/tooling/extensibility docs,
   - coding steward/eval docs,
   - remote/control docs,
   - testing convention docs.
5. Replace remaining compatibility paths with short bridge pages only where
   active references require them.
6. Run OpenSpec strict validation and stale-reference checks.
