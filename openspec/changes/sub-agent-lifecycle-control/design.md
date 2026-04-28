## Context

Issues #312-#319 come from the same operational failure mode: delegated work exists, but the parent runtime and human operator do not have a reliable lifecycle record for it. The current child runtime path in `crates/runtime/src/runtime/child_agents.rs` launches a child, waits for terminal events, and uses a single timeout sleep. `crates/runtime/src/runtime/virtual_tools.rs` then persists a bounded delegated result whose summary is silently capped at 320 characters. Memory surfaces in `crates/runtime/src/runtime/memory_surfaces.rs` use arbitrary character truncation, and daemon/TUI surfaces have no child-run control plane.

The target contract is already documented in `docs/spec/sub_agent_lifecycle_contract.md`, and the issue map is tracked in `plans/2026-04-28-sub-agent-lifecycle-memory-plan.md`. This OpenSpec change turns that contract into implementation slices.

## Goals / Non-Goals

**Goals:**

- Make every delegated child launch observable through a child-run record before initial submission.
- Use heartbeat/progress freshness for child timeout classification instead of only launch wall-clock duration.
- Give daemon, TUI, and parent runtime paths the same termination state transition.
- Preserve completed child output fidelity through full output fields or inspectable references plus explicit preview/truncation metadata.
- Keep generated memory and `.alan` runtime state coherent, human-readable, and out of normal source status.
- Canonicalize workspace identity paths sufficiently to avoid Alan-home `.alan/.alan` nesting and case-variant duplicates.

**Non-Goals:**

- Replace the spawn/skill execution contract wholesale.
- Stream all child tokens into the parent tape.
- Build durable recovery for child-run records beyond the metadata already persisted in sessions/rollouts.
- Make generated memory files a lossless transcript store.
- Remove support for source-controlled `.alan/agents/default/`, `.alan/agents/<name>/`, policies, or authored skills.

## Decisions

### Child-Run Registry Lives In Runtime, Mirrored By Daemon

Add runtime-owned child-run metadata types and a registry handle that child launch/supervision updates directly. Parent runtime execution needs this state before the daemon sees it, so the runtime is the source of truth for lifecycle transitions. The daemon exposes read/list/terminate APIs by querying runtime manager state for active sessions and by returning the last known metadata stored in session state.

Alternative considered: daemon-only registry keyed by session events. That would miss child launches before events are bridged and would not help parent-runtime virtual tools terminate children without going through HTTP.

### Liveness Uses Idle Timeout Plus Optional Wall-Clock Cap

Child supervision tracks `latest_progress_at` and `latest_heartbeat_at`. Any child event for the active submission updates progress. A heartbeat ticker updates heartbeat while the child controller is waiting, so long-running but live children do not trip the delegated timeout. `timeout_secs` becomes the idle timeout. A larger wall-clock cap can be derived as a guard, but timeout classification is based on idle freshness.

Alternative considered: simply increase `timeout_secs`. That only masks the issue and still cannot distinguish dead children from quiet but healthy children.

### Termination Is A Shared Lifecycle Transition

Daemon endpoints, TUI commands, and the parent virtual tool all call one runtime-manager termination path with `{child_run_id, actor, mode, reason}`. Graceful termination calls child shutdown/cancellation first; forceful termination aborts. The child-run record moves through `terminating` to `terminated` or remains terminal if it was already complete.

Alternative considered: implement separate TUI-only and model-only kill paths. That would produce divergent audit semantics and make lifecycle tests brittle.

### Result Handoff Separates Preview From Output

Keep a compact tape payload, but change the delegated result shape so truncation is explicit. A completed child result contains `summary`, optional `summary_preview`, `child_run`, `output_text` or `output_ref`, optional structured output/ref, and a `truncation` object. Rollout payloads keep enough metadata to retrieve full text from child rollout when the tape preview is bounded.

Alternative considered: store full child output directly in every parent tool message. That preserves fidelity but increases parent tape size and makes compaction worse.

### Memory Surfaces Are Agent-Authored Through The Memory Skill

Semantic memory surfaces should be produced by the active agent through the `memory` skill or another explicit agent-visible memory workflow. That lets the same governed turn decide durable decisions, constraints, open loops, references, and next actions without adding an uncontrolled runtime-internal model call. Runtime still owns validation, path/budget enforcement, persistence, and deterministic fallback rendering.

Deterministic line-aware truncation remains the fallback when the skill is unavailable, the agent does not author a summary, memory writes fail, cancellation prevents a final summary, or hard safety budgets require a bounded generated surface.

Alternative considered: hidden runtime LLM summarization. That would preserve more semantics than truncation, but it introduces an ungoverned model call outside the agent's visible skill/tool flow. Another alternative was only using Markdown-aware truncation. That prevents broken fragments but still lets length, not semantic importance, decide what survives.

### Path Hygiene Uses Existing Workspace Resolver Boundaries

Fix Alan-home nesting and path canonicalization in `workspace_resolver`, `registry`, runtime path helpers, and runtime manager comparisons rather than inventing a new workspace identity service. Generated `.alan` runtime paths are ignored by repo rules, while authored agent roots remain trackable.

Alternative considered: make all `.alan/` ignored. That would block workspace-local agent definitions and policies from being intentionally committed.

## Risks / Trade-offs

- Registry only in memory for active runtimes -> expose rollout/session references in terminal records and preserve enough payload metadata for debugging after runtime shutdown.
- Heartbeat ticker may report liveness while a child is stuck in a long tool call -> treat heartbeat as runtime liveness, not user-visible progress, and keep optional wall-clock cap plus active tool/progress metadata.
- Result payload shape changes can affect prompt expectations -> keep `status` and `summary` stable, add fields compatibly, and update skill injection/docs.
- Skill-authored memory summaries depend on agent behavior -> keep deterministic fallback rendering, make the memory skill explicit about continuation summaries, and never block turn completion on a memory surface failure.
- TUI command scope can grow quickly -> implement the minimum commands from the contract and keep formatting text-only.
- Path canonicalization differs by filesystem -> canonicalize when the path exists, normalize stored identities, and test case behavior conditionally where the platform supports it.

## Migration Plan

1. Add OpenSpec specs/tasks and keep `docs/spec/sub_agent_lifecycle_contract.md` as the narrative target contract.
2. Implement runtime child-run metadata, liveness, result payload, and memory/path fixes with focused unit tests.
3. Add daemon endpoints and TUI commands once runtime state is available.
4. Update docs and `.gitignore`.
5. Run `cargo test` for touched crates and `bun test` for TUI command/client coverage.

Rollback is source-level: the change adds metadata and endpoints compatibly, so reverting the implementation and generated OpenSpec artifacts restores the prior behavior.

## Open Questions

- Whether child-run metadata should be persisted as its own file format or remain recoverable through rollout/session state in the first implementation.
- The exact wall-clock cap default when `timeout_secs` is small or absent.
- Whether parent-runtime `terminate_child_run` should be included in all virtual tool sets or only when delegated skills are available.
