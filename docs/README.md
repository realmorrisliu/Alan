# Alan Documentation Index

This repository now treats documentation as a small set of explicit categories:

1. Current implementation and operator guides.
2. Target contracts and product specs.
3. Maintainer-only operational notes.
4. Active implementation plans.
5. Historical plans and idea/rationale documents.

The `Status` line at the top of each document is the authority boundary. Do not
read a `VNext` or `target` document as a statement about shipped behavior unless
it explicitly says so.

## Start Here

- [Architecture](./architecture.md)
- [Current Governance Contract](./governance_current_contract.md)
- [Specs Index](./spec/README.md)
- [Plans Index](../plans/README.md)

## Current Behavior And Guides

These are the best entry points for understanding what the repository
guarantees today.

- [Architecture](./architecture.md)
- [Current Governance Contract](./governance_current_contract.md)
- [Skills And Tools](./skills_and_tools.md)
- [Skill Authoring](./skill_authoring.md)
- [Testing Strategy](./testing_strategy.md)

Important current-vs-target pairs:

- Governance today: [governance_current_contract.md](./governance_current_contract.md)
- Governance target design: [policy_over_sandbox.md](./policy_over_sandbox.md)
- Skill-system stable contract: [spec/skill_system_contract.md](./spec/skill_system_contract.md)
- Skill-system current implementation guide: [skills_and_tools.md](./skills_and_tools.md)

## Specs And Contracts

Use these indexes instead of treating `docs/spec/` as a flat bucket:

- [Spec Index](./spec/README.md)
- [Maintainer Docs](./maintainer/README.md)
- [Plans Index](../plans/README.md)

## Design Rationale And Accepted Direction

These documents explain why the contracts look the way they do.

- [Alphabet Design](./alphabet_design.md)
- [Autonomy Layered Design](./autonomy_layered_design.md)
- [Policy Over Sandbox](./policy_over_sandbox.md)

## Validation And Harness

- [Harness Overview](./harness/README.md)
- [Harness Self-Eval](./harness/self_eval/README.md)
- [Harness KPI](./harness/metrics/kpi.md)

## Philosophy, Directions, And Ideas

- [Human In The End](./human_in_the_end.md)
- [Long-Term Directions](./directions/README.md)
- [Idea Backlog](./ideas/README.md)
