# HITE Governance (V2)

> Status: accepted target design for Alan's next governance revision.  
> This document is not the authoritative description of today's implementation.
> For current behavior, see
> [`governance_current_contract.md`](../governance_current_contract.md).

## Why

Alan follows HITE's core flow:

`Human Defines -> Agent Executes -> Human Owns`

The point of governance is not to turn the human into a step-by-step approver.
The point is to let the human define intent, trust boundaries, budgets, and
owner-only decisions up front, then let the agent operate autonomously inside
those bounds.

Under this model:

1. `Policy` decides whether an action is authorized.
2. The runtime either allows, denies, or escalates that action.
3. Any available execution backend applies defense-in-depth restrictions.
4. Humans re-enter the loop only at true boundaries or after the outcome.

Strong containment can still exist, but it is not Alan's core HITE mechanism.
Alan should remain coherent as an owner-operated local agent system even when no
strict OS sandbox is present.

Current implementation note:

- the current backend is `workspace_path_guard`, which is a best-effort
  execution guard rather than a strict OS sandbox
- policy files replace builtin profile rules; they are not implicitly merged
- `tool_escalation` is reserved for `PolicyEngine::Escalate`
- runtime side-effect replay confirmations use `effect_replay_confirmation`

---

## Core Thesis

HITE governance in Alan means:

1. Humans define the ends:
   - objective
   - trusted scope
   - budgets
   - irreversible boundaries
   - owner-only actions
2. Agents choose the means:
   - routine coding and exploration should not require constant approvals
   - the agent should be free to replan within the declared boundary
3. The runtime manages exceptions:
   - deny unsafe or unauthorized actions
   - escalate owner-boundary decisions explicitly
   - preserve auditability and accountability
4. Humans own outcomes:
   - review the result package, not every keystroke
   - inspect boundary hits, denials, and side effects when needed

This is governance over authorization semantics, not governance through
micromanagement.

---

## Governance Model

Each tool call is evaluated in this order:

1. Classify tool capability (`read | write | network | unknown`).
2. Evaluate `PolicyEngine` rule match.
3. Apply policy action:
   - `allow`: execute through the available host execution backend
   - `deny`: block execution and return a recoverable tool result
   - `escalate`: emit `Event::Yield` for explicit human decision
4. If execution proceeds, the host execution backend may still enforce
   additional restrictions.

There is no `approval_policy` downgrade path in V2. Escalation behavior is
explicit and uniform.

The important distinction is:

- `allow` means "authorized"
- `deny` means "not authorized"
- `escalate` means "this is an owner boundary"

Those semantics should hold whether the host has only a lightweight guard or a
strong containment backend.

---

## Trust Boundaries And Owner Boundaries

Alan should reason about boundaries that matter to intent and ownership, not
just filesystem paths.

Examples:

- trusted repo vs unknown external code
- local branch vs shared branch or `main`
- agent-created resource vs shared production resource
- internal service vs external exfiltration target
- reversible workspace edits vs irreversible deletion or publish actions

Typical owner boundaries include:

- production deploys
- destructive data deletion
- force-push or history rewrite outside the agent's working branch
- sharing data to a new external destination
- security-posture changes
- actions whose target was inferred rather than explicitly grounded

Ambiguous authorization should not be treated as implicit permission.

---

## Deny And Escalate Semantics

### `deny`

`deny` is a normal control signal, not a fatal mode switch.

When policy denies an action:

1. execution does not happen
2. the denial is returned as a tool result or recoverable boundary outcome
3. the agent is expected to replan in good faith rather than route around the
   boundary

This keeps long-running sessions autonomous without turning every policy miss
into a hard stop.

### `escalate`

`escalate` is reserved for owner-boundary decisions where the agent should stop
and ask for a human judgment.

When escalation occurs, runtime must emit `Yield` with enough context for a
real decision:

- `request_id`
- `action_summary`
- `risk_reason`
- `boundary_type`
- `suggested_options`
- optional constraints or safer alternatives

No silent downgrade is allowed once boundary flow starts.

---

## Containment And Execution Backends

Containment is defense in depth, not Alan's primary HITE control plane.

Today:

- the built-in backend is `workspace_path_guard`
- it provides best-effort workspace/path/process-shape validation
- it is not a hard security boundary

Target direction:

- Alan may support optional stronger containment backends for deployments that
  need them, especially shared, remote, or enterprise environments
- those backends must be additive host capabilities, not the definition of HITE
- owner-operated local workflows must not depend on strong containment to make
  governance coherent

In other words:

- lack of strong containment does not mean lack of governance
- presence of containment does not replace policy

---

## Policy File

When `governance.policy_path` is not provided, workspace policy is resolved from
the `AgentRoot` chain. Default workspace agents use:

- `~/.alan/agents/default/policy.yaml -> {workspace}/.alan/agents/default/policy.yaml`

Named agents extend that chain with:

- `~/.alan/agents/<name>/policy.yaml -> {workspace}/.alan/agents/<name>/policy.yaml`

Current file shape remains:

```yaml
rules:
  - id: deny-prod-delete
    tool: bash
    match_command: "kubectl delete"
    action: deny
    reason: protect shared production state

  - id: escalate-publish
    tool: bash
    match_command: "git push origin main"
    action: escalate
    reason: owner boundary for publish

default_action: allow
```

`PolicyFile` currently deserializes only `rules` and `default_action`; extra
fields are ignored.

Longer-term policy evolution should enrich authorization semantics with fields
such as:

- trust boundary
- ownership boundary
- reversibility
- blast-radius ceiling
- budget guard

---

## Audit Requirements

Every governance decision must be traceable in rollout/events:

- `policy_source`
- `rule_id`
- `action` (`allow | deny | escalate`)
- `reason`
- capability classification
- trust or owner-boundary context when available
- effective execution backend
- resolver (`policy | human`)
- side-effect references or outcome summary when relevant

This is required for HITE-style outcome ownership.

---

## Acceptance Criteria

1. Alan's default governance story works without requiring a strict OS sandbox.
2. Routine local coding work rarely interrupts the human.
3. Boundary hits are explicit, auditable, and ownership-aware.
4. Ambiguous authorization does not silently become permission.
5. Optional stronger containment, when present, does not change core governance
   semantics.
