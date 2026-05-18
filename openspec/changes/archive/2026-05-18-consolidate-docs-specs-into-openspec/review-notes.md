# Documentation Consolidation Review Notes

## Deleted Historical Surfaces

These files were execution history or completed checklists, not durable
contract sources. Any still-current decision was already represented in an
active OpenSpec change, long-lived OpenSpec spec, current implementation guide,
or current package path.

| Path | Reason |
| --- | --- |
| `docs/maintainer/session_reconnect_fix_checklist.md` | Completed historical checklist; no longer guides current operation. |
| `docs/superpowers/plans/2026-05-15-sidebar-local-space-swipe.md` | Superseded by macOS shell OpenSpec specs and archived/active shell changes. |
| `docs/superpowers/specs/2026-05-15-sidebar-local-space-swipe-design.md` | Superseded by macOS shell OpenSpec specs and archived/active shell changes. |
| `docs/superpowers/plans/2026-05-18-terminal-input-pipeline.md` | Manual verification was moved into `fix-macos-terminal-interaction-regressions/verification.md`. |
| `plans/2026-04-13-provider-compat-execution-plan.md` | Implemented provider/request-control work; durable behavior lives in OpenSpec provider specs. |
| `plans/2026-04-18-alan-coding-steward-boundary-plan.md` | Implemented repo-coding package boundary; current package path and OpenSpec owners exist. |
| `plans/2026-04-09-tui-adaptive-ux-plan.md` | Superseded by `replace-typescript-tui-with-rust-inline-tui`. |
| `plans/2026-04-19-repo-worker-package-migration-plan.md` | Package migration is present under `crates/runtime/skills/repo-coding/` with repo-worker harness paths. |
| `plans/2026-04-28-sub-agent-lifecycle-memory-plan.md` | Covered by child-run, delegated-result, runtime-memory, workspace-hygiene, and hardening OpenSpec owners. |
| `plans/README.md` | Removed because `plans/` no longer owns active sequencing. |

## Bridged Legacy `docs/spec` Surfaces

Every legacy `docs/spec/*.md` contract page is now a short compatibility bridge
instead of a long-form contract. The bridge names the OpenSpec owner and avoids
restating the old requirements.

| Legacy path | OpenSpec owner |
| --- | --- |
| `docs/spec/alan_shell_macos_contract.md` | `macos-shell-terminal-lifecycle`, `macos-shell-workspace-interactions`, `macos-shell-control-plane-reliability`, `macos-shell-ui-ux-conformance`, `macos-shell-build-test-contract`, `macos-terminal-runtime-foundation`, `macos-terminal-surface-parity` |
| `docs/spec/alan_macos_shell_ui_ux.md` | `macos-shell-ui-ux-conformance`, `macos-shell-workspace-interactions`, `macos-shell-build-test-contract` |
| `docs/spec/daemon_api_contract.md` | `daemon-api-contract` plus active daemon deltas in hardening, anywhere, and Rust TUI changes |
| `docs/spec/app_server_protocol.md` | `daemon-api-contract`, `runtime-core-contract`, active anywhere/hardening daemon deltas |
| `docs/spec/kernel_contract.md`, `docs/spec/execution_model.md`, `docs/spec/durable_run_contract.md`, `docs/spec/scheduler_contract.md`, `docs/spec/interaction_inbox_contract.md`, `docs/spec/compaction_contract.md` | `runtime-core-contract`, `runtime-memory-contract`, `human-visible-run-lifecycle`, `runtime-memory-surfaces` |
| `docs/spec/memory_architecture.md`, `docs/spec/pure_text_memory_contract.md` | `runtime-memory-contract`, `runtime-memory-surfaces`, `runtime-memory-write-audit` |
| `docs/spec/provider_capability_contract.md`, `docs/spec/provider_auth_contract.md`, `docs/spec/connection_profile_contract.md`, `docs/spec/provider_reasoning_effort_migration.md` | `provider-connection-contract`, `provider-request-controls`, `openrouter-provider-adapter` |
| `docs/spec/hite_governance.md`, `docs/spec/governance_boundaries.md`, `docs/spec/tool_catalog_binding_contract.md`, `docs/spec/capability_router.md`, `docs/spec/extension_contract.md`, `docs/spec/workspace_routing_contract.md` | `governance-tooling-contract`, `agent-capability-routing`, `skill-system-contract`, `agent-root-layout`, `workspace-runtime-state-hygiene` |
| `docs/spec/skill_system_contract.md` | `skill-system-contract` |
| `docs/spec/alan_coding_steward_contract.md`, `docs/spec/alan_coding_governance_contract.md`, `docs/spec/alan_coding_eval_contract.md` | `coding-steward-contract`, `runtime-harness-contract`, `agent-capability-routing`, `delegated-result-handoff`, `runtime-evidence-provenance` |
| `docs/spec/sub_agent_lifecycle_contract.md` | `child-run-lifecycle`, `delegated-result-handoff`, `runtime-memory-surfaces`, `workspace-runtime-state-hygiene`, active hardening lifecycle deltas |
| `docs/spec/remote_control_architecture.md`, `docs/spec/remote_control_security.md` | `remote-control-contract`, `alan-anywhere`, `daemon-api-contract` |
| `docs/spec/harness_bridge.md` | `runtime-harness-contract`, `daemon-api-contract`, lifecycle hardening deltas |
| `docs/spec/rust_test_placement_contract.md` | `rust-test-placement-contract` |
| `docs/spec/alan_tui_ui_ux.md` | `replace-typescript-tui-with-rust-inline-tui` / `rust-inline-tui` |

## OpenSpec Deltas Added

This change adds OpenSpec deltas for the legacy domains that did not already
have a clear long-lived owner:

- `documentation-governance`
- `runtime-core-contract`
- `runtime-memory-contract`
- `provider-connection-contract`
- `governance-tooling-contract`
- `skill-system-contract`
- `coding-steward-contract`
- `remote-control-contract`
- `runtime-harness-contract`
- `rust-test-placement-contract`

The existing macOS, daemon, provider-request, memory-surface, child-run, and
workspace-hygiene specs remain owners where they already existed.

## Retained Non-OpenSpec Docs

These stay outside OpenSpec because they are current guides, runbooks,
validation instructions, or executable fixture documentation rather than
durable product/runtime specs.

| Path | Reason |
| --- | --- |
| `docs/architecture.md` | Architecture guide; normative links now point to OpenSpec. |
| `docs/skills_and_tools.md` | Tool/skill implementation and operator guide; normative links now point to OpenSpec. |
| `docs/skill_authoring.md` | Authoring workflow guide; normative package behavior points to OpenSpec. |
| `docs/testing_strategy.md` | Testing guide; Rust placement and harness contracts point to OpenSpec. |
| `docs/governance_current_contract.md` | Current implementation guide for shipped governance semantics. |
| `docs/live_provider_harness.md`, `docs/live_runtime_smoke.md` | Opt-in live validation guides. |
| `docs/harness/README.md`, `docs/harness/self_eval/README.md`, `docs/harness/metrics/kpi.md` | Runner/fixture documentation; reusable harness behavior points to OpenSpec. |
| `docs/harness/scenarios/**/*.json` | Executable fixtures, not prose specs. |
| `docs/maintainer/*.md` | Current maintainer runbooks; active contract references now point to OpenSpec. |
