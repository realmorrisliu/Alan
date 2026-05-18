## Why

alan currently has two competing places for durable specifications:
`docs/spec/` and `openspec/specs/`. That makes review and implementation
ambiguous because a reader must decide whether a Markdown file under `docs/`,
an OpenSpec long-lived spec, or an active OpenSpec change is authoritative.

This change makes OpenSpec the only source of truth for spec management and
turns non-OpenSpec docs back into implementation guides, maintainer runbooks,
validation instructions, fixtures, or short bridge pointers.

## What Changes

- Add a documentation-governance capability that defines where durable
  contracts, in-flight design, implementation guides, runbooks, validation
  data, and historical plans belong.
- Migrate durable contract content out of `docs/spec/` into OpenSpec
  long-lived specs or active OpenSpec changes.
- Replace still-linked `docs/spec/` contract files with short bridge pointers
  only where a compatibility link is needed during migration.
- Delete historical Superpowers and `plans/` execution plans once their active
  decisions are captured in OpenSpec or current implementation guides.
- Keep current implementation guides, maintainer runbooks, harness fixtures,
  and live validation instructions outside OpenSpec when they do not define
  normative product/runtime behavior.
- Update repository indexes and agent guidance so future spec work is created
  under OpenSpec instead of `docs/spec/`, `plans/`, or `docs/superpowers/`.

## Capabilities

### New Capabilities

- `documentation-governance`: defines alan's documentation source-of-truth
  taxonomy, OpenSpec-only spec management rule, bridge-page policy, historical
  plan cleanup policy, and validation expectations for doc/spec drift.
- `runtime-core-contract`: owns the migrated kernel, execution, durable-run,
  scheduler, compaction, interaction-inbox, and app-server protocol contract
  material that did not already have a long-lived OpenSpec owner.
- `runtime-memory-contract`: owns the migrated memory architecture and
  pure-text memory contract material.
- `provider-connection-contract`: owns the migrated provider capability,
  provider/auth, and connection-profile contract material.
- `governance-tooling-contract`: owns migrated HITE governance, tool catalog,
  capability routing, extension, and workspace-routing contract material.
- `skill-system-contract`: owns migrated skill package, discovery, exposure,
  execution, and management contract material.
- `coding-steward-contract`: owns migrated coding steward, repo-worker,
  coding governance, and coding eval contract material.
- `remote-control-contract`: owns migrated remote architecture and security
  contract material.
- `runtime-harness-contract`: owns migrated harness bridge, runner, KPI, and
  self-eval contract material.
- `rust-test-placement-contract`: owns migrated Rust test placement contract
  material.

### Modified Capabilities

- `macos-shell-build-test-contract`: require macOS shell documentation,
  architecture notes, and active OpenSpec tasks to point at OpenSpec long-lived
  specs rather than resurrecting `docs/spec/` as a parallel contract surface.

## Impact

- Affected docs: `docs/README.md`, `docs/spec/README.md`, `docs/spec/*.md`,
  `docs/superpowers/**`, `plans/**`, and selected current implementation
  guides that link to old spec locations.
- Affected OpenSpec areas: a new `documentation-governance` long-lived spec
  and a small delta to `macos-shell-build-test-contract`.
- No runtime API or product behavior changes are intended.
- Verification requires OpenSpec strict validation plus link/path checks for
  stale references to `docs/spec/`, `plans/`, and `docs/superpowers/` as
  contract sources.
