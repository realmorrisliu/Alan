# Governance Boundaries (Commit Boundaries)

> Status: VNext governance contract (execution plane for Human-in-the-End exception handling).

## Goals

Upgrade from "approve every step" to "take over at commit boundaries":

`Human Defines -> Agent Executes -> Human Owns`

The core objective is to make high-risk irreversible actions declarative, enforceable, and auditable.

## Boundary Levels

### Level A: Routine

- Low-risk, reversible, low-impact actions.
- Default policy: `allow`.

Examples: file reads, local static analysis, temporary draft generation.

### Level B: Sensitive

- Side effects exist but are controllable; may affect quality or cost.
- Default policy: `escalate` or constrained `allow`.

Examples: bulk file writes, external network calls, non-production deploys.

### Level C: Commit Boundary

- High-risk, irreversible, legal/financial/production impact.
- Default policy: `escalate` (or `deny` when needed).

Examples: production release, real payments, deletion of critical data, push to main branch.

## Risk Dimensions

Policy evaluation should include at least:

1. Capability type (`read/write/network/unknown`).
2. Target scope (paths, environment, resource domain).
3. Blast radius (file count, changed LOC, target systems).
4. Reversibility (can it be rolled back?).
5. Cost/budget (time, tokens, money).

## Policy-as-Code Extensions

On top of `allow/deny/escalate`, recommended fields:

1. `risk_level`: `A/B/C`
2. `boundary`: whether this is a commit boundary
3. `requires_owner`: whether owner-level confirmation is required
4. `max_impact`: blast-radius ceiling
5. `budget_guard`: cost threshold

These fields may come from policy files and/or runtime-enriched calculations.

## Interaction Contract

When `escalate` is hit:

1. Runtime must emit `Yield` with:
   - `request_id`
   - `action_summary`
   - `risk_reason`
   - `suggested_options`
2. External side returns explicit decision through `Resume`: allow/deny + optional constraints.

No silent downgrade is allowed after entering boundary flow.

## Audit Chain Requirements

Each boundary decision should record:

1. `policy_source` (`builtin/workspace/custom`)
2. `rule_id`
3. `risk_level`
4. `action` (`allow/deny/escalate`)
5. `reason`
6. `request_id`
7. `resolver` (`human/agent/policy`)
8. `resolved_at`

## Relationship with Sandbox

1. Policy decides whether action should happen.
2. Sandbox decides whether action can physically happen.

Policy must never expand sandbox boundaries, only tighten or escalate.

## Relationship with Outcome Ownership

Under HITE, humans are not button operators but outcome owners.

Governance should support:

1. Defining boundaries and budgets before execution.
2. Minimal intervention on exceptions.
3. Post-hoc accountability through audit trails.

## Minimal Rollout Path

1. Define 10-20 high-value boundary rules (production, money, deletion, push).
2. Route all boundary hits through unified `Yield/Resume`.
3. Extend rollout with governance audit fields.
4. Add regression scenarios for boundary policies.

## Acceptance Criteria

1. High-risk actions never bypass confirmation.
2. Low-risk actions are not over-blocked (avoid approval fatigue).
3. Every boundary decision is traceable to rule, reason, and ownership.
