# Multi-Model Deliberation

> Status: speculative idea backlog entry.
> Purpose: preserve a design direction for later validation, not to define current Alan behavior.

## Summary

Alan may benefit from a multi-model execution pattern where:

1. Smaller, faster models handle narrow, low-risk, high-volume subtasks.
2. Larger models handle planning, coordination, ambiguity resolution, and final judgment.
3. The system upgrades from cheap execution to expensive deliberation only when needed.

This is best understood as a compute-budget and control-layer design, not as literal
simulation of Daniel Kahneman's "System 1 / System 2" psychology.

## Working Hypothesis

Instead of one model handling every phase of an agent turn, Alan could separate:

1. Fast path:
   low-cost routing, classification, extraction, retrieval summarization, local codebase search,
   supporting subagent work, simple drafts.
2. Deliberate path:
   multi-step planning, tool orchestration, conflict resolution, side-effect review,
   final answer synthesis, and high-risk decisions.

The architecture value would come from selective escalation, not from anthropomorphic role-play.

## Why This Might Matter For Alan

Alan already has runtime ingredients that could support this direction:

1. Turn execution is already explicit and state-machine driven.
2. Tool orchestration, governance, and yield/resume create natural escalation boundaries.
3. Skills define workflow logic while runtime owns invariants and recovery.
4. Harness infrastructure already exists for profile comparison and regression gating.

This suggests that a future multi-model path, if pursued, should be expressed as:

1. routing policy
2. escalation rules
3. evaluation thresholds

and not as a personality prompt that asks one model to "be intuitive" and another to "be rational."

## Candidate Division Of Labor

Potential future shape:

1. `nano` or similarly cheap model:
   triage, extraction, ranking, simple tool-result condensation, narrow subagents.
2. `mini` model:
   medium-difficulty coding subtasks, codebase exploration, document review, intermediate synthesis.
3. frontier model:
   planning, final judgment, ambiguity handling, recovery after failures, risky or expensive actions.

This should remain policy-driven. The system should decide when to upgrade rather than hard-code one
model per user-visible task category.

## Main Risks

1. Router overhead can erase cost or latency wins if escalation is too frequent.
2. Small-model errors can poison later stages if outputs are trusted too early.
3. Added complexity can reduce debuggability unless routing decisions are observable.
4. Benchmark gains may fail to translate into end-to-end agent improvements.
5. Long-context and high-ambiguity tasks may need large-model involvement earlier than expected.

## Validation Questions

Before treating this as a roadmap item, Alan should answer:

1. Does multi-model routing improve end-to-end task success, not just component metrics?
2. Which subtasks are reliably "small-model safe" inside Alan workflows?
3. What escalation signals are predictive enough to keep failure rates low?
4. How much orchestration overhead is acceptable before the design stops paying for itself?
5. Can harness scenarios detect regressions in routing quality, duplicate side effects, and recovery?

## Evidence To Revisit Later

Relevant external directions worth revisiting when this idea becomes active:

1. LLM cascades and cost-quality routing.
2. Multi-LLM routers and model selection benchmarks.
3. Strong-verifier patterns for weaker reasoners.
4. Product evidence from coding-subagent systems using mixed model sizes.

## Promotion Criteria

This idea should only move out of the backlog if all of the following become true:

1. There is a concrete Alan use case that is bottlenecked by single-model cost, latency, or reliability.
2. A routing policy can be expressed at the runtime/daemon/skill boundary without muddying contracts.
3. Harness scenarios exist to compare single-model and multi-model profiles.
4. The resulting design is explainable enough to preserve Alan's small-kernel philosophy.
