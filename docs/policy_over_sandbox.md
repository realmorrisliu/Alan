# Policy Over Sandbox (V2)

> Status: accepted breaking design for Alan governance model.  
> This document is the target contract for the next protocol/runtime revision.

## Why

Alan follows HITE's core flow:

`Human Defines -> Agent Executes -> Human Owns`

In runtime terms:

1. `Policy` decides what should happen now.
2. `Sandbox` enforces what is physically executable.
3. `Steering` (`Op::Input`) can interrupt long-running execution.

"Policy over sandbox" means policy is the first-class decision layer, while sandbox is an execution boundary, not a user-facing approval mode.

---

## Governance Model

Each tool call is evaluated in this order:

1. Classify tool capability (`read | write | network | unknown`).
2. Evaluate `PolicyEngine` rule match.
3. Apply policy action:
   - `allow`: execute tool in sandbox backend
   - `deny`: block execution and emit recoverable error/tool result
   - `escalate`: emit `Event::Yield` for explicit human decision
4. If execution proceeds, sandbox backend still enforces file/network/process constraints.

There is no `approval_policy` downgrade path in V2. Escalation behavior is explicit and uniform.

---

## Breaking Changes

V2 removes compatibility governance knobs:

- remove `ApprovalPolicy`
- remove `SandboxMode`
- remove all `on_request/never` compatibility branching in runtime policy logic

Session/runtime configuration moves to a single governance entry:

```json
{
  "governance": {
    "profile": "autonomous",
    "policy_path": ".alan/policy.yaml"
  }
}
```

`profile` provides builtin defaults; `policy_path` overrides with workspace policy-as-code.

---

## Steering as First-Class Control

`Op::Input` is treated as steering, not as a normal queued turn.

During a tool batch:

1. After each tool completion, runtime polls in-band steering queue.
2. If steering exists, remaining tool calls in that batch are marked `skipped_due_to_steering`.
3. Steering messages are injected before the next LLM generation.
4. Execution continues in the same turn state instead of starting a new task.

This guarantees responsiveness without breaking tape/turn consistency.

---

## Policy File

Workspace policy lives at `{workspace}/.alan/policy.yaml`.

```yaml
rules:
  - id: deny-prod-delete
    tool: bash
    match_command: "kubectl delete"
    action: deny
    reason: protect production cluster

  - id: escalate-prod-deploy
    tool: bash
    match_command: "deploy --prod"
    action: escalate
    reason: deployment boundary

default_action: allow
```

`PolicyFile` currently deserializes only `rules` and `default_action`; extra fields are ignored.

Rule fields:

- `tool`: tool name or `*`
- `capability`: `read | write | network | unknown` (optional)
- `match_command`: bash substring match, case-insensitive (optional)
- `action`: `allow | deny | escalate`
- `reason`: optional audit reason

Matching rule: first match wins, then `default_action`.

---

## Builtin Profiles

### `autonomous` (default)

- allow by default
- deny only critical destructive patterns
- escalate only for explicitly marked boundaries

### `conservative`

- deny network by default
- escalate write and unknown capability
- allow read by default

Profiles are only presets. Effective behavior is always the resolved policy file + rule set.

---

## Sandbox Role in V2

Sandbox is not a policy mode switch. It is an execution backend:

- path/workspace boundary checks
- optional OS-level process sandbox
- protected subpaths under writable roots (e.g. `.git`, `.alan`, `.agents`)
- plain shell commands with statically addressable paths only under the workspace path-guard backend; shell control flow is rejected, common wrapper forms (`env`, `command`, `builtin`, `exec`, `time`, `nice`, `nohup`, `timeout`, `stdbuf`, `setsid`) are rejected, protected process paths are blocked conservatively, glob patterns are rejected, direct nested evaluators are rejected, direct opaque command dispatchers (for example `xargs` and `find -exec`) are rejected, and a curated set of common direct script interpreters (for example `python file.py`, `bash script.sh`, and `awk -f script.awk`) are rejected. The backend validates explicit path-like argv references and shell redirection targets, but it does not infer utility-specific operand roles for arbitrary bare tokens. Arbitrary program-internal writes or dispatch are also not inspected by this backend, including commands that mutate private state without an explicit path operand (for example `git init`, `git add`, or `git config --local`), utility actions like `find -delete`, and utility-specific script/DSL modes such as build/task runners or `sed -f`; those still require policy escalation or a stronger OS sandbox if you need execution-boundary guarantees.

Policy can tighten behavior but never widen sandbox-enforced boundaries.

---

## Audit Requirements

Every tool decision must be traceable in rollout/events:

- `policy_source` (builtin/workspace)
- `rule_id`
- `action` (`allow|deny|escalate`)
- `reason`
- capability classification
- effective sandbox backend

This is required for HITE-style outcome ownership.
