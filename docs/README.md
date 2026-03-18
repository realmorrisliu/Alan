# Alan Documentation Index

## How to Read

Follow this order:

1. Kernel and execution contracts: define core invariants and boundaries.
2. Main architecture contracts: define target direction and current baseline.
3. Design RFCs: explain migration rationale and tradeoffs.
4. Validation documents: define quality gates and regression strategy.
5. Philosophy essays: explain decision principles and long-term intent.

## 1) Kernel / Execution Contracts (Highest Priority)

- `docs/spec/kernel_contract.md`
- `docs/spec/execution_model.md`
- `docs/spec/app_server_protocol.md`
- `docs/spec/governance_boundaries.md`

These are the primary source of truth for runtime behavior.

## 2) Mainline Architecture Contracts (Target + Current)

- `docs/spec/durable_run_contract.md`
- `docs/spec/scheduler_contract.md`
- `docs/spec/interaction_inbox_contract.md`
- `docs/spec/compaction_contract.md`
- `docs/spec/memory_architecture.md`
- `docs/spec/capability_router.md`
- `docs/spec/extension_contract.md`
- `docs/spec/harness_bridge.md`
- `docs/spec/remote_control_architecture.md`
- `docs/spec/remote_control_security.md`

These documents define the VNext layering and rollout path.

## 3) Design RFCs (Migration Explanations)

- `docs/alphabet_design.md`
- `docs/autonomy_layered_design.md`

Use these when evaluating why a contract exists, not only what it says.

## 4) Validation System Documents

- `docs/testing_strategy.md`
- `docs/harness/README.md`

These define test layers, protocol drift prevention, and release gates.

## 5) Philosophy Essays

- `docs/human_in_the_end.md`

Use these to align product decisions with Alan's operator model.

## 6) Maintainer Operations

- `docs/maintainer/github_automation.md`
- `docs/maintainer/remote_control_phased_plan.md`

Use this for repository governance, GitHub automation, and contributor-ready setup.

## 7) Long-Term Directions (Intent, Non-Contract)

- `docs/directions/README.md`
- `docs/directions/apprenticeship_learning.md`

Use these to capture strategic directions that are expected to matter to Alan over time,
but are still too early for contracts, rollout plans, or implementation commitments.

## 8) Exploration / Idea Backlog (Non-Contract)

- `docs/ideas/README.md`
- `docs/ideas/multi_model_deliberation.md`

Use these to capture speculative directions, hypotheses, and ideas that should not be
treated as committed architecture or roadmap.
