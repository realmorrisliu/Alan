# Reference Coding Agent (Product Layer on Alan Runtime)

> Status: VNext reference implementation spec (no runtime fork).

## Goal

Provide a runnable coding-agent reference built on top of Alan runtime primitives, validating:

1. Kernel genericity (no coding-only runtime branch logic).
2. Product-layer composition via profile + skills + extensions.
3. Harness-regressable coding behavior for recovery/governance/safety.

## Layer Responsibilities

### Kernel (`alan-runtime` + daemon)

1. Turn state machine and input mode semantics.
2. Scheduler, checkpoint, dedupe, governance boundaries.
3. Tool orchestration + policy/sandbox enforcement.

### Product Layer (Reference Coding Agent)

1. Coding profile defaults (toolset + governance baseline).
2. Coding skill pack (decompose, edit, verify, deliver).
3. Extension slots (code index, test analysis, PR helper).
4. Coding-focused harness scenarios and thresholds.

### Harness Layer

1. Validate long coding loop continuity.
2. Validate steer/follow_up/next_turn behavior in coding flow.
3. Validate restart recovery + side-effect dedupe + boundary handoff.

## Minimum Coding Loop

Target loop (MVP):

1. Receive coding task.
2. Plan and decompose into actionable steps.
3. Apply code change through tools.
4. Run verification commands.
5. Produce delivery summary with risk/test status.

## Extension and Router Alignment

The reference coding agent must stay compatible with:

1. `extension_contract.md`
2. `capability_router.md`
3. `harness_bridge.md`
4. `provider_auth_contract.md`

Design constraint:

1. Builtin and extension capabilities are selected through one routing path.
2. Bridge-hosted capabilities are optional and additive.

## Input Mode Behavior in Coding Workflows

1. `steer`: re-plan active coding loop quickly (skip remaining safe steps as needed).
2. `follow_up`: queue additional changes for immediate next cycle.
3. `next_turn`: queue future coding intent/context without immediate execution.

## Provider / Auth Path

The reference coding agent's first realistic OpenAI-family execution path should align with
`provider_auth_contract.md`.

Normative expectations:

1. API Platform access remains available through `openai_*` providers.
2. ChatGPT/Codex subscription access is exposed through a distinct `chatgpt` provider surface.
3. The reference coding agent must consume this provider path without introducing coding-only
   branches into the kernel.
4. Provider-specific auth/account behavior must remain outside prompt-layer coding skills.

## Durability and Governance Requirements

1. Unfinished coding runs resume after restart via checkpoint restore.
2. Irreversible side effects are deduped via idempotency/effect records.
3. Unknown side-effect status forces governance-safe escalation (no bypass).

## Scaffold Artifacts (Repository)

Reference scaffold is provided under:

1. `reference/coding-agent/`
2. `scripts/reference/run_coding_reference_smoke.sh`
3. `scripts/harness/run_coding_reference_suite.sh`
4. `docs/harness/scenarios/coding/*`

## Quick Validation Commands

1. `bash scripts/reference/run_coding_reference_smoke.sh --mode local`
2. `bash scripts/harness/run_coding_reference_suite.sh --ci-blocking`

## Acceptance Mapping

This spec + scaffold targets issue acceptance:

1. Runnable reference coding flow without runtime fork.
2. Coding behavior implemented in skills/extensions/product layer.
3. Input mode semantics validated in coding scenarios.
4. Restart recovery + dedupe validated by harness scripts.
5. Regressions can be blocked via coding harness gate.
