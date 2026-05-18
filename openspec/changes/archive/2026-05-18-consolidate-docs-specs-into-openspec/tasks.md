## 1. Establish Documentation Governance

- [x] 1.1 Add or update the long-lived `documentation-governance` OpenSpec capability during archive sync.
  - 2026-05-18: `openspec archive consolidate-docs-specs-into-openspec
    --yes` created `openspec/specs/documentation-governance/spec.md`.
- [x] 1.2 Update `docs/README.md` so it states that durable specs live in OpenSpec, not `docs/spec/`.
- [x] 1.3 Replace `docs/spec/README.md` with a short migration bridge or remove it after active links are updated.
- [x] 1.4 Update `AGENTS.md` and top-level README references that describe `docs/spec/` as the authoritative spec index.

## 2. Migrate `docs/spec/` Contracts In Domain Slices

- [x] 2.1 Move macOS shell contract material from `docs/spec/alan_shell_macos_contract.md` and the superseded UI/UX bridge into existing macOS OpenSpec capabilities or short bridge pages.
- [x] 2.2 Move runtime/kernel/execution/compaction/memory contract material into OpenSpec capabilities, creating new capabilities only where no durable owner exists.
- [x] 2.3 Move provider, auth, connection, and request-control material into OpenSpec provider capabilities and remove duplicate `docs/spec` contract text.
- [x] 2.4 Move governance, capability-router, extension, tool-catalog, and workspace-routing material into OpenSpec capabilities.
- [x] 2.5 Move coding steward, coding governance, coding eval, TUI UX, remote control, harness bridge, and Rust test placement contracts into OpenSpec capabilities.
- [x] 2.6 For each migrated file, either delete the old `docs/spec/*.md` path or replace it with a short bridge that names the OpenSpec replacement.

## 3. Clean Historical Plan Surfaces

- [x] 3.1 Delete implemented or superseded `docs/superpowers/` plans after confirming their active decisions are already in OpenSpec.
- [x] 3.2 Move any still-current decisions from `docs/superpowers/plans/2026-05-18-terminal-input-pipeline.md` into `openspec/changes/fix-macos-terminal-interaction-regressions/` before deleting the plan.
- [x] 3.3 Delete implemented or superseded `plans/*.md` files after migrating any still-current decisions into OpenSpec changes/specs.
  - 2026-05-18: Deleted implemented `2026-04-13-provider-compat-execution-plan.md`
    and `2026-04-18-alan-coding-steward-boundary-plan.md`; retained active or
    partially landed TUI, repo-worker, and sub-agent/memory plans as migration
    inputs.
  - 2026-05-18: Deleted the remaining TUI, repo-worker, and sub-agent/memory
    plans after mapping them to active OpenSpec changes or landed package
    paths.
- [x] 3.4 Remove or rewrite `plans/README.md` once `plans/` no longer owns active implementation sequencing.

## 4. Preserve Current Guides And Runbooks

- [x] 4.1 Keep `docs/architecture.md`, `docs/skills_and_tools.md`, `docs/skill_authoring.md`, and `docs/testing_strategy.md` only as current implementation guides, with normative links pointing to OpenSpec.
- [x] 4.2 Keep `docs/live_provider_harness.md` and `docs/live_runtime_smoke.md` as opt-in validation guides.
- [x] 4.3 Keep maintainer runbooks that still guide current operations, and delete completed historical checklists.
- [x] 4.4 Keep `docs/harness/scenarios/**/*.json`, self-eval docs, and KPI docs as executable fixtures and runner documentation, with any normative harness behavior captured in OpenSpec.

## 5. Validation And Drift Checks

- [x] 5.1 Run `openspec validate consolidate-docs-specs-into-openspec --type change --strict --json`.
- [x] 5.2 Run `openspec validate --all --strict --json`.
- [x] 5.3 Search active non-archived docs and guidance for stale references to `docs/spec/`, `plans/`, or `docs/superpowers/` as authoritative contract sources.
- [x] 5.4 Run `git diff --check`.
- [x] 5.5 Re-run focused macOS shell documentation/build-test checks if macOS shell README, architecture, or build/test metadata changes.
  - 2026-05-18: No `clients/apple` README, architecture, or build/test
    metadata changes were made by this cleanup. Focused stale-reference search
    for retired macOS shell contract paths found no active authoritative
    references outside the cleanup change and bridge notes.

## 6. Review And Archive Readiness

- [x] 6.1 Review the final diff to confirm it does not include unrelated Apple runtime changes from the current worktree.
  - 2026-05-18: Reviewed the cleanup pathset separately from the dirty
    worktree. Existing Apple/runtime and release-script diffs remain outside
    this documentation consolidation scope and should not be staged with it.
- [x] 6.2 Prepare review notes listing each deleted, bridged, migrated, and retained documentation surface with the reason.
- [x] 6.3 After implementation is merged, sync accepted delta specs into `openspec/specs/`.
  - 2026-05-18: Archive sync created the new long-lived specs and updated
    `macos-shell-build-test-contract`.
- [x] 6.4 Run full OpenSpec validation after sync and archive this change.
  - 2026-05-18: Full OpenSpec validation passed after archive sync, and this
    change was archived under
    `openspec/changes/archive/2026-05-18-consolidate-docs-specs-into-openspec/`.
