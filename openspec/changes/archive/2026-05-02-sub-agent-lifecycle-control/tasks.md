## 1. Runtime Child-Run Lifecycle

- [x] 1.1 Add child-run status, liveness, termination, and handoff metadata types to runtime/protocol surfaces.
- [x] 1.2 Add an in-memory child-run registry owned by parent runtime state and register child runs before initial submission.
- [x] 1.3 Update child event observation to refresh latest progress, heartbeat, warnings, active tool/plan summaries, and terminal metadata.
- [x] 1.4 Replace child supervision hard timeout with idle-timeout semantics and an optional larger wall-clock cap.
- [x] 1.5 Add runtime tests for healthy long-running heartbeat, idle timeout, failure, cancellation, and terminal status transitions.

## 2. Daemon Control Plane

- [x] 2.1 Expose runtime-manager APIs to list/read child-run records for a parent session.
- [x] 2.2 Add daemon routes for list child runs, read child run, and terminate child run.
- [x] 2.3 Route graceful and forceful termination through one shared child-run lifecycle transition.
- [x] 2.4 Add daemon tests for list/read/terminate, unknown child id, already-terminal child, and timed-out child metadata.

## 3. Parent Runtime Termination Tool

- [x] 3.1 Add `terminate_child_run` virtual tool definition with child id, reason, and graceful/forceful mode.
- [x] 3.2 Apply governance/audit semantics and record termination actor/reason/mode on child-run records.
- [x] 3.3 Add virtual tool tests for successful termination, unknown child id, already-terminal child, and denied/escalated policy behavior.

## 4. Delegated Result Handoff

- [x] 4.1 Extend delegated result and invocation record types with child-run reference, output text/ref, preview fields, and truncation metadata.
- [x] 4.2 Update completed, failed, timed-out, cancelled, paused, and terminated child handoff construction.
- [x] 4.3 Keep parent tape payload bounded while preserving full output in rollout/reference metadata.
- [x] 4.4 Update skill injector/docs so parent prompts know how to inspect full child output.
- [x] 4.5 Add tests for long output, short output, structured output truncation, and timeout metadata.

## 5. Memory Surface Reliability

- [x] 5.1 Update the memory skill/docs so semantic continuation summaries are agent-authored through the visible skill/tool flow.
- [x] 5.2 Replace arbitrary character truncation fallback with line/Markdown-aware truncation helpers and explicit omission markers.
- [x] 5.3 Add source session/rollout references to fallback memory truncation markers where available.
- [x] 5.4 Refresh working memory, handoff, session summary, and daily notes after terminal turn state is final.
- [x] 5.5 Add memory tests for long Markdown output, code fences, truncation markers, and terminal plan-state refresh.

## 6. TUI Child-Agent Management

- [x] 6.1 Add TUI client methods and types for child-run list/read/terminate APIs.
- [x] 6.2 Implement `/agents`, `/agent <id>`, `/agent terminate <id> [reason]`, and `/agent kill <id> [reason]`.
- [x] 6.3 Surface active child-run count in the compact runtime HUD when available.
- [x] 6.4 Update `/help` and add TUI tests for command parsing and client calls.

## 7. Runtime State And Path Hygiene

- [x] 7.1 Update `.gitignore` to ignore generated `.alan/sessions/` and generated `.alan/memory/` runtime files while keeping agent definitions trackable.
- [x] 7.2 Fix workspace resolver/runtime path handling so Alan home does not create nested `.alan/.alan` state.
- [x] 7.3 Canonicalize workspace identity comparisons and generated workspace ids where paths exist.
- [x] 7.4 Add safe detection/reporting for legacy nested Alan-home runtime state.
- [x] 7.5 Update docs explaining generated `.alan` paths versus source-controlled agent roots.
- [x] 7.6 Add tests or repo checks for ignore patterns, Alan-home path resolution, and path casing normalization where practical.

## 8. Verification

- [x] 8.1 Run focused Rust tests for runtime child agents, virtual tools, memory surfaces, daemon routes, workspace resolver, and registry.
- [x] 8.2 Run focused TUI tests for client and command handling.
- [x] 8.3 Run `openspec status --change sub-agent-lifecycle-control` and record final implementation status.
