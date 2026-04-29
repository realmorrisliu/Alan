# Sub-Agent Lifecycle Contract

> Status: target runtime, daemon, and TUI contract for delegated child
> runtimes, result handoff, and runtime-owned state surfaces.

## Goal

Make delegated sub-agent execution observable, controllable, and recoverable
without forcing parent runtimes or humans to infer child state from hard
timeouts or truncated memory files.

The target outcome is:

1. child runtimes can prove they are still making progress,
2. parent runtimes and TUI operators can list, inspect, and terminate child
   work explicitly,
3. delegated results preserve enough information for parent integration,
4. runtime memory and workspace state surfaces remain useful debugging aids
   instead of lossy or misleading artifacts.

## Non-Goals

This contract does not:

1. define a strict OS sandbox,
2. replace the `SpawnSpec` launch contract,
3. require child runtimes to stream every token into the parent tape,
4. require `.alan/memory/` to become a lossless transcript store,
5. turn workspace `.alan/agents/default/` configuration into disposable runtime state.

## Stable Vocabulary

- **Parent runtime**: the runtime that launches and supervises one or more child
  runtimes.
- **Child runtime**: a fresh delegated runtime launched through an explicit
  spawn or delegated-skill contract.
- **Child run**: the observable lifecycle record for one child runtime launch.
- **Heartbeat**: a runtime-owned liveness signal emitted by a child while it is
  still working, even when no user-visible text is produced.
- **Progress signal**: a heartbeat plus optional status such as current plan
  item, active tool, recent warning, or latest event cursor.
- **Idle timeout**: a timeout measured from the latest heartbeat or progress
  signal.
- **Wall-clock cap**: an optional maximum runtime duration used only as a final
  resource guard.
- **Termination**: an explicit operator or parent-runtime request to stop a
  child run, recorded with actor, reason, and mode.
- **Result handoff**: the bounded result object the parent receives after a
  child reaches a terminal or blocked state.
- **Runtime-owned state**: `.alan/sessions/`, `.alan/memory/working/`, handoffs,
  daily notes, and other generated files written by the runtime.

## Child Run Lifecycle

Every child runtime launch must create or register a child run record before
the first task submission is sent to the child.

A child run record must include:

1. child run id,
2. child session id,
3. parent session id,
4. target workspace root when known,
5. launch target and delegated skill metadata when applicable,
6. rollout path when available,
7. created time,
8. current status,
9. latest heartbeat time,
10. latest event cursor or sequence when available,
11. latest compact status summary when available.

Child run statuses are:

1. `starting`
2. `running`
3. `blocked`
4. `completed`
5. `failed`
6. `timed_out`
7. `terminating`
8. `terminated`
9. `cancelled`

Terminal statuses are `completed`, `failed`, `timed_out`, `terminated`, and
`cancelled`.

## Liveness And Timeout Semantics

Child supervision should prefer liveness over hard wall-clock expiry.

Rules:

1. Child runtimes must emit progress signals while active.
2. Any child event observed by the parent updates `latest_event_at`.
3. A periodic heartbeat updates `latest_heartbeat_at` even when the model or
   tool has not emitted user-visible output.
   Heartbeats are internal supervision signals and are not appended to the
   user-visible session event stream or replay buffer.
4. Parent supervision must not classify a child as timed out while fresh
   heartbeat or progress signals are still arriving.
5. Idle timeout is measured from the latest heartbeat or progress signal.
6. Wall-clock cap is optional and should be substantially larger than the idle
   timeout.
7. When idle timeout expires, the child run becomes `timed_out` and the result
   handoff must include child run metadata and the latest known progress.
8. If the runtime actually stops the process, status must distinguish
   `timed_out` from operator-requested `terminated`.

The child should not be considered healthy solely because its process exists.
The source of truth is runtime-observed heartbeat or progress.

## Control Plane

The daemon must expose a control plane that can:

1. list child runs for a parent session,
2. read a child run by id,
3. terminate a child run with a reason,
4. expose enough rollout and workspace metadata for debugging.

The parent runtime should also be able to request child termination through a
governed runtime tool. That tool must record:

1. child run id,
2. requesting actor,
3. reason,
4. whether termination was graceful or forceful,
5. resulting status.

Human and model-initiated termination must share the same runtime state
transition path.

## TUI Contract

`alan-tui` must expose child-run management as operator commands.

Minimum commands:

1. `/agents` lists child runs for the current session, grouped by status.
2. `/agent <id>` shows child run details, including status, workspace, latest
   heartbeat, current plan or active tool, and rollout path.
3. `/agent terminate <id> [reason]` requests graceful termination.
4. `/agent kill <id> [reason]` requests forceful termination when graceful
   termination is unavailable or stuck.

The TUI should also surface active child-run counts in the compact runtime HUD
when a parent session has children.

## Delegated Result Handoff

The delegated result object returned to the parent must preserve integration
fidelity without flooding the parent tape.

Required fields:

1. `status`
2. `summary`
3. `summary_preview` when the full summary is too large for the parent tape
4. `child_run`
5. `output_text` or `output_ref`
6. `structured_output` or `structured_output_ref`
7. `truncation` metadata when any field is shortened
8. `warnings`
9. `error_kind` and `error_message` when applicable

Rules:

1. A completed child must not be reduced to an unlabeled 320-character preview.
2. If the full output is too large to inline, the result must include an
   inspectable reference such as child session id, rollout path, and output
   cursor.
3. The parent must be able to tell the difference between "the child produced
   only a short answer" and "the child produced a long answer that was
   intentionally summarized".
4. Structured output truncation must preserve critical status and summary keys
   and include explicit truncation metadata.

## Memory Surface Rules

Runtime-owned memory surfaces are curated continuation aids, not arbitrary
string previews.

Rules:

1. Memory surfaces must not cut Markdown headings, bullets, or code fences in
   the middle of a token without an explicit truncation marker.
2. Truncation markers must say what was omitted and where to inspect the source
   rollout.
3. Session-summary and handoff surfaces must refresh after terminal turn state
   is known.
4. A completed turn must not leave active plan items in `in_progress` unless
   the runtime has evidence the work is intentionally still open.
5. Working memory may stay compact, but it must remain semantically coherent.

Raw rollout files remain the source of truth for event replay. Memory surfaces
must point to rollouts when they omit relevant detail.

## Workspace Runtime State

Generated runtime state must not pollute normal repo status.

Rules:

1. `.alan/sessions/` is runtime-owned generated state.
2. `.alan/memory/working/` is runtime-owned generated state.
3. `.alan/memory/handoffs/`, `.alan/memory/daily/`, and
   `.alan/memory/sessions/` are generated episodic state unless a workspace
   deliberately opts into tracking them.
4. `.alan/agents/default/` is the workspace default agent definition root.
   `.alan/agents/<name>/` contains workspace named agent definition roots.
   These authored definitions, policies, or skills may be source-controlled
   when the project wants workspace-local agent configuration.
5. Repository templates should ignore generated runtime state by default while
   allowing opt-in tracking for agent definitions.

## Home Workspace And Path Canonicalization

Alan home must not accidentally become a nested workspace that creates
`~/.alan/.alan/` runtime state.

Rules:

1. When the workspace root is Alan home, runtime state paths must resolve to
   the canonical Alan home layout instead of appending another `.alan/`.
2. Workspace identity comparisons must use canonical paths where the platform
   can provide them.
3. Case variants such as `/Users/name/Developer/Alan` and
   `/Users/name/Developer/alan` must not produce separate workspace identities
   on case-insensitive filesystems.
4. Migration or cleanup tooling should detect legacy nested Alan-home state and
   report it before removal.

## Relationship To Other Contracts

1. `workspace_routing_contract.md` defines when parent runtimes should route
   local work into child runtimes.
2. `alan_coding_steward_contract.md` defines the coding steward / repo-worker
   product boundary.
3. `pure_text_memory_contract.md` defines memory layers and file ownership.
4. `alan_tui_ui_ux.md` defines the operator console layout and summary-surface
   behavior.
5. `execution_model.md` defines the task, run, session, and turn hierarchy.

## Acceptance Criteria

This contract is satisfied when:

1. active child runtimes emit heartbeat or progress signals,
2. delegated child supervision uses idle timeout rather than a single hard
   wall-clock timeout,
3. parent runtimes and the TUI can list and inspect child runs,
4. parent runtimes and the TUI can terminate child runs explicitly,
5. completed child output is not silently collapsed into a short preview,
6. memory surfaces no longer contain broken fragments such as half-rendered
   Markdown bullets,
7. terminal session summaries reflect completed plan state,
8. generated `.alan` runtime state is ignored or clearly separated from
   source-controlled workspace agent definitions,
9. Alan home does not create nested `.alan/.alan` runtime state,
10. workspace identity is canonical enough to avoid duplicate state for path
    casing variants on case-insensitive filesystems.
