# Sub-Agent Lifecycle And Memory Reliability Plan (2026-04-28)

> Status: active tracking plan for the child-runtime lifecycle, TUI
> management, delegated result handoff, and runtime-state hygiene issues found
> while inspecting root session `441b634c-7344-45c8-bdb4-e45bf7b9868e`.

## Why This Plan Exists

The root session delegated TUI repository inspection to child runtimes. That
surfaced several related reliability problems:

1. one child timed out and was reduced to `child_timed_out`,
2. the successful child produced a full answer, but the parent only received a
   320-character delegated summary,
3. runtime memory surfaces contained broken fragments such as
   `### Top-level directories` followed by `- c...`,
4. memory and handoff files still showed `in_progress` after the task store
   marked the session complete,
5. generated workspace `.alan/` state appeared as untracked repository files,
6. Alan home contained nested `.alan/.alan/` runtime state.

The target contract is `docs/spec/sub_agent_lifecycle_contract.md`.

## Scope

This plan tracks eight issues:

1. child liveness and idle timeout,
2. child-run lifecycle registry and daemon control plane,
3. TUI child-agent management commands,
4. governed parent-runtime child termination tool,
5. delegated result handoff fidelity,
6. memory surface truncation and terminal-state refresh,
7. workspace `.alan` generated-state hygiene,
8. Alan-home nesting and path canonicalization.

## Current Evidence

Observed in root session `441b634c-7344-45c8-bdb4-e45bf7b9868e`:

1. `/Users/morris/.alan/sessions/rollout-20260428-111136-441b634c-7344-45c8-bdb4-e45bf7b9868e.jsonl`
   recorded child `abde85e1-3eea-493d-88fd-6acbfd293fc8` as
   `terminal_status=timed_out`.
2. The same root rollout recorded child
   `d54d4327-ce5f-475b-af60-ce76d389f4d8` as completed, but the delegated
   result summary was exactly 320 characters.
3. `.alan/sessions/rollout-20260428-111528-d54d4327-ce5f-475b-af60-ce76d389f4d8.jsonl`
   contains a full final assistant message of 5582 characters.
4. `.alan/memory/working/d54d4327-ce5f-475b-af60-ce76d389f4d8.md` contains a
   broken fragment ending with `- c...`.
5. `/Users/morris/.alan/memory/working/441b634c-7344-45c8-bdb4-e45bf7b9868e.md`
   still includes `in_progress` plan state after task completion.
6. `git status --short --ignored .alan` reports `?? .alan/`.
7. `/Users/morris/.alan/.alan/` exists with generated runtime directories.

Relevant code baseline:

1. `crates/runtime/src/runtime/child_agents.rs` uses a single timeout sleep and
   calls `abort_runtime()` on expiry.
2. `crates/runtime/src/runtime/virtual_tools.rs` caps delegated result summary
   persistence at `MAX_DELEGATED_RESULT_SUMMARY_CHARS = 320`.
3. `crates/runtime/src/runtime/memory_surfaces.rs` caps inline text at
   `MAX_INLINE_TEXT_CHARS = 280` and appends `...`.
4. `clients/tui/src/index.tsx` exposes `/sessions`, `/interrupt`,
   `/compact`, and `/rollback`, but no child-agent management commands.

## Issue Tracking

1. [#312](https://github.com/realmorrisliu/Alan/issues/312) -
   runtime/protocol, P0:
   Replace child hard timeout with heartbeat-backed idle timeout.
2. [#313](https://github.com/realmorrisliu/Alan/issues/313) -
   runtime/daemon/protocol, P0:
   Add child-run lifecycle registry and daemon control plane.
3. [#314](https://github.com/realmorrisliu/Alan/issues/314) - tui, P1:
   Add `/agents` and `/agent ...` management commands.
4. [#315](https://github.com/realmorrisliu/Alan/issues/315) -
   runtime/governance, P1:
   Add governed parent-runtime child termination tool.
5. [#316](https://github.com/realmorrisliu/Alan/issues/316) -
   runtime/protocol, P0:
   Preserve delegated result handoff fidelity beyond 320-char previews.
6. [#317](https://github.com/realmorrisliu/Alan/issues/317) -
   runtime/memory, P0:
   Fix memory-surface truncation and completed-session refresh.
7. [#318](https://github.com/realmorrisliu/Alan/issues/318) -
   repo/config, P1:
   Ignore generated workspace `.alan` runtime state by default.
8. [#319](https://github.com/realmorrisliu/Alan/issues/319) -
   runtime/paths, P1:
   Prevent Alan-home `.alan/.alan` nesting and canonicalize workspace paths.

## Delivery Order

Recommended sequence:

1. land the contract and issue map,
2. implement child-run registry and heartbeat events,
3. convert delegated supervision to idle-timeout semantics,
4. add termination control paths,
5. expose child management in the TUI,
6. fix delegated result payload truncation,
7. fix memory-surface rendering and terminal refresh,
8. clean up runtime-state path and ignore rules.

This order makes lifecycle state available before TUI and termination features
depend on it.

## Implementation Slices

### Slice 1: Child Liveness And Registry

Files likely affected:

1. `crates/protocol/src/event.rs`
2. `crates/protocol/src/spawn.rs`
3. `crates/runtime/src/runtime/child_agents.rs`
4. `crates/runtime/src/runtime/engine.rs`
5. `crates/alan/src/daemon/session_store.rs`
6. `crates/alan/src/daemon/routes.rs`

Exit criteria:

1. child runs are registered before launch,
2. active children report heartbeat/progress,
3. idle timeout uses latest progress time,
4. daemon can list and read child-run state.

### Slice 2: Termination Control

Files likely affected:

1. `crates/runtime/src/runtime/virtual_tools.rs`
2. `crates/runtime/src/runtime/tool_orchestrator.rs`
3. `crates/alan/src/daemon/routes.rs`
4. `crates/alan/src/daemon/runtime_manager.rs`
5. `docs/governance_current_contract.md`

Exit criteria:

1. human and parent-runtime termination share one state transition path,
2. termination records actor and reason,
3. graceful and forceful termination are distinguishable,
4. terminated children produce inspectable terminal child-run records.

### Slice 3: TUI Child-Agent Management

Files likely affected:

1. `clients/tui/src/client.ts`
2. `clients/tui/src/index.tsx`
3. `clients/tui/src/summary-surfaces/runtime-state.ts`
4. `clients/tui/src/summary-surfaces/hud-surface.tsx`
5. `clients/tui/src/client.test.ts`

Exit criteria:

1. `/agents` lists children for the current session,
2. `/agent <id>` shows details,
3. `/agent terminate <id> [reason]` requests graceful termination,
4. `/agent kill <id> [reason]` requests forceful termination,
5. runtime HUD shows active child count when relevant.

### Slice 4: Result Handoff Fidelity

Files likely affected:

1. `crates/runtime/src/runtime/virtual_tools.rs`
2. `crates/runtime/src/runtime/child_agents.rs`
3. `crates/runtime/src/skills/types.rs`
4. `crates/runtime/src/runtime/virtual_tools_tests.rs`
5. `docs/skills_and_tools.md`

Exit criteria:

1. completed child output is not silently reduced to 320 characters,
2. full child output is available inline or by reference,
3. truncation metadata is explicit,
4. parent prompts can distinguish short output from shortened output.

### Slice 5: Memory Surface Reliability

Files likely affected:

1. `crates/runtime/src/runtime/memory_surfaces.rs`
2. `crates/runtime/src/runtime/turn_executor.rs`
3. `crates/runtime/src/runtime/memory_flush.rs`
4. `crates/runtime/src/runtime/turn_state.rs`
5. `docs/spec/pure_text_memory_contract.md`

Exit criteria:

1. memory surfaces truncate by coherent sections or lines,
2. truncation markers point to source rollouts,
3. terminal refresh writes completed plan state,
4. handoff and session summaries do not keep stale `in_progress` items after
   successful completion.

### Slice 6: Runtime State Hygiene

Files likely affected:

1. `.gitignore`
2. `crates/runtime/src/paths.rs`
3. `crates/runtime/src/agent_root.rs`
4. `crates/alan/src/registry.rs`
5. `docs/architecture.md`

Exit criteria:

1. generated `.alan/sessions/` and `.alan/memory/` runtime files are ignored by
   default,
2. workspace `.alan/agents/default/` and `.alan/agents/<name>/` remain available
   for source-controlled agent definitions,
3. Alan home does not create nested `.alan/.alan/` state,
4. path casing variants resolve to one workspace identity where supported.

## Review Heuristics

Review this track against these questions:

1. Can an operator tell whether a child is working, blocked, or dead without
   reading raw rollout files?
2. Can the parent stop a child without treating timeout as the only control
   path?
3. Does the parent receive enough child output to make a faithful final answer?
4. Are memory files readable by humans and free of misleading half-fragments?
5. Does repo status stay focused on source changes rather than generated Alan
   state?
