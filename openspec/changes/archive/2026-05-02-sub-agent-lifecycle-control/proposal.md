## Why

Delegated child runtimes currently fail as opaque side effects: parents see hard timeouts, lossy 320-character summaries, and generated memory/runtime files that are hard to trust. Issues #312-#319 need one coordinated change because liveness, child-run state, result handoff, memory surfaces, and workspace path hygiene all affect whether a parent runtime or operator can safely supervise delegated work.

## What Changes

- Add first-class child-run lifecycle state with heartbeat/progress timestamps, terminal status, rollout metadata, and daemon read/list/control APIs.
- Replace delegated child hard timeout behavior with idle-timeout semantics backed by child heartbeat/progress, with an optional larger wall-clock cap as a resource guard.
- Add explicit child termination paths shared by daemon/TUI operators and governed parent-runtime tooling, recording actor, reason, mode, and final state.
- Preserve delegated result fidelity by separating full output from bounded previews and adding explicit truncation/reference metadata.
- Make semantic memory/handoff summaries an agent-visible memory skill behavior, with runtime line/section-aware fallback rendering after completed turn state is known.
- Ignore generated workspace `.alan` runtime state by default while keeping authored agent roots trackable.
- Canonicalize workspace identities enough to prevent Alan-home `.alan/.alan` nesting and path-case duplicates on case-insensitive filesystems.

## Capabilities

### New Capabilities

- `child-run-lifecycle`: Child runtimes expose lifecycle records, liveness, timeout classification, daemon control, TUI inspection, and governed termination.
- `delegated-result-handoff`: Delegated child results preserve full output or inspectable references while keeping previews bounded and labeled.
- `runtime-memory-surfaces`: Agent-authored memory and handoff summaries preserve semantic state, while generated fallback surfaces truncate coherently and reflect terminal turn/plan state.
- `workspace-runtime-state-hygiene`: Generated `.alan` runtime state is separated from source-controlled agent definitions and workspace identity paths are canonicalized.

### Modified Capabilities

- None.

## Impact

- Runtime/protocol: child event handling, spawn supervision, virtual tools, result payloads, rollout references, memory refresh.
- Daemon API: child-run list/read/terminate endpoints and runtime-manager/session-store state.
- TUI: `/agents`, `/agent <id>`, `/agent terminate`, `/agent kill`, help text, client calls, compact HUD child count.
- Repository/docs: `.gitignore`, generated-state documentation, path-resolution behavior.
- Tests: runtime lifecycle and timeout tests, daemon route tests, TUI command parsing/client tests, memory skill/fallback surface tests, and path hygiene tests.
