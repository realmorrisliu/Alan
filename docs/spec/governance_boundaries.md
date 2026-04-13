# HITE Governance Boundaries

> Status: VNext governance contract (execution plane for Human-in-the-End
> exception handling).

## Goal

Replace "approve every step" with boundary-driven intervention:

`Human Defines -> Agent Executes -> Human Owns`

The contract here is about authorization and ownership, not about requiring a
strong sandbox before Alan can be useful.

## Boundary Classes

### Level A: Routine

- Low-risk, reversible, local actions.
- Default policy: `allow`.

Examples:

- file reads
- local search and analysis
- reversible source edits inside the working scope

### Level B: Sensitive

- Side effects exist and may affect quality, cost, or external state.
- Default policy: constrained `allow`, `deny`, or `escalate` depending on trust
  boundary and blast radius.

Examples:

- broad file rewrites
- external network calls
- non-production deploys
- actions against shared but non-critical resources

### Level C: Owner Boundary

- High-risk, irreversible, externally visible, or ownership-sensitive actions.
- Default policy: `escalate`, or `deny` when outside declared intent.

Examples:

- production release
- real payments
- deletion of critical data
- force-push or push to `main`
- sharing data to a new external destination

## Risk Dimensions

Policy evaluation should include at least:

1. Capability type (`read | write | network | unknown`).
2. Target scope (paths, environment, resource domain).
3. Trust boundary (local repo, internal service, external destination, shared
   infrastructure).
4. Blast radius (file count, changed LOC, target systems, number of resources).
5. Reversibility.
6. Cost or budget impact.
7. Authorization clarity.

Authorization ambiguity matters explicitly. If the system cannot tell whether
the user authorized the real blast radius of the action, it must not infer
permission silently.

## Policy-as-Code Extensions

On top of `allow | deny | escalate`, recommended fields are:

1. `risk_level`: `A | B | C`
2. `trust_boundary`
3. `owner_boundary`
4. `requires_owner`
5. `max_impact`
6. `budget_guard`

These fields may come from policy files and/or runtime-enriched calculations.

## Interaction Contract

### `deny`

`deny` means the action is not authorized.

Requirements:

1. Execution does not happen.
2. The denial is returned as a recoverable runtime result.
3. The agent may replan, but must not attempt to route around the boundary in
   bad faith.

### `escalate`

`escalate` means the action crosses an owner boundary and needs explicit human
judgment.

When escalation is hit:

1. Runtime must emit `Yield` with:
   - `request_id`
   - `action_summary`
   - `risk_reason`
   - `boundary_type`
   - `suggested_options`
2. External side returns explicit decision through `Resume`:
   - `allow`
   - `deny`
   - optional constraints

No silent downgrade is allowed after entering boundary flow.

## Audit Chain Requirements

Each boundary decision should record:

1. `policy_source` (`builtin | workspace | custom`)
2. `rule_id`
3. `risk_level`
4. `action` (`allow | deny | escalate`)
5. `reason`
6. `request_id`
7. `resolver` (`human | policy`)
8. `resolved_at`
9. execution backend identity

## Relationship With Execution Backends

1. Policy decides whether an action is authorized.
2. Any available execution backend decides what the host will physically allow.

Governance semantics must remain coherent even when no strict containment backend
exists. Optional stronger containment may tighten execution, but it does not
define HITE itself.

## Relationship With Outcome Ownership

Under HITE, humans are not button operators but outcome owners.

Governance should support:

1. Defining boundaries and budgets before execution.
2. Minimal intervention on true exceptions.
3. Post-hoc accountability through audit trails and side-effect summaries.

## Minimal Rollout Path

1. Define high-value boundary rules for production, publish, deletion, external
   sharing, and credential misuse.
2. Route all owner-boundary hits through unified `Yield/Resume`.
3. Extend rollout with governance audit fields.
4. Add regression scenarios for ambiguity, exfiltration, and ownership
   boundaries.

## Acceptance Criteria

1. High-risk owner-boundary actions never bypass explicit handling.
2. Low-risk work is not over-blocked.
3. Every boundary decision is traceable to rule, reason, and ownership.
4. Governance remains meaningful in owner-local mode without assuming a strict
   OS sandbox.
