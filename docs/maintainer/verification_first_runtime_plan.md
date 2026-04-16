# Verification-First Runtime Plan

> Status: maintainer implementation plan for verification-first behavior,
> runtime contradiction recovery, and durable payload hygiene.

> Owner issue: `#262`

## Why

Recent sessions exposed a recurring class of failures:

1. The agent claimed current/external data was unavailable without first trying
   an available tool.
2. The model had injected persona context, but the on-disk path grounding was
   weak enough that it still guessed the wrong file path when it chose to
   verify or persist.
3. `bash` capability classification over-reported `write` for read-only shell
   control patterns such as `cd ... && ls`, polluting governance semantics.
4. Rollout persistence stored raw network payloads that are useful during the
   current turn but too sensitive for durable audit by default.

Alan already has the right philosophy for these issues:

1. Files are the source of truth, not implicit model memory.
2. Policy/governance semantics belong to runtime, not prompt hope.
3. Rollout is an auditable record, not a raw secret dump.

This note defines the concrete implementation split to align current behavior
with those principles.

## Goals

1. Make verification-first behavior the default product behavior for
   current/external-state questions.
2. Ground persona/user-context file operations in explicit runtime facts rather
   than inferred relative paths.
3. Preserve HITE-style governance semantics by improving `bash` capability
   classification where current behavior is plainly wrong.
4. Prevent clearly contradictory capability claims from being emitted to users.
5. Keep rollout auditability while redacting sensitive durable payload fields.

## Non-Goals

1. Do not hardcode product-specific tool choices into kernel/runtime.
2. Do not introduce a full semantic shell interpreter.
3. Do not add a new top-level spec before implementation proves the shape.
4. Do not add `ToolCapability::Unknown` in this batch; that is a follow-up once
   current misclassifications are reduced and behavior is stable.

## Design Principles

### 1. Prompt/Profile Nudges Improve First Attempt Quality

Prompt/profile guidance should encourage the model to probe before making
capability claims, but it is not the source of truth and is not enough on its
own.

### 2. Runtime Must Ground Facts It Already Knows

If runtime already knows the resolved persona overlay path or the writable
overlay target, that fact should be injected explicitly instead of asking the
model to infer it from a file name.

### 3. Runtime Manages Contradictions Before Emission

If an assistant draft claims current/external data access is unavailable while
the session has relevant tools, that is a runtime contradiction. It should be
recovered before the user sees the text.

### 4. Governance Inputs Must Be Honest

Capability classification is the first step in governance. Plainly read-only
shell control patterns must not be mislabeled as `write`.

### 5. Durable Audit Is Not Raw Payload Persistence

The current turn may need full live payloads. Durable rollout storage should
persist an auditable, redacted view instead of every secret-bearing byte.

## PR Split

### PR1: Verification-First Prompting, Persona Grounding, and Bash Classification

#### Scope

1. Strengthen verification-first prompt guidance.
2. Inject explicit persona file grounding (`resolved_from`, `write_to`) into the
   workspace persona context.
3. Fix obviously incorrect `bash` capability classification for shell-local
   control + read-only command chains.
4. Add prompt/unit/self-eval coverage for the new behavior.

#### Primary Files

1. `crates/runtime/prompts/runtime_base.md`
2. `crates/runtime/prompts/persona/AGENTS.md`
3. `crates/runtime/src/prompts/workspace.rs`
4. `crates/runtime/src/prompts/assembler.rs`
5. `crates/runtime/src/agent_definition.rs` (only if additional grounded data
   is easier to expose here)
6. `crates/runtime/src/agent_root.rs` (overlay path helpers if needed)
7. `crates/tools/src/lib.rs`
8. `docs/harness/self_eval/README.md`
9. `docs/harness/scenarios/profiles/{baseline,candidate}/...` as needed

#### Expected Code Changes

1. Add explicit “probe before claim” guidance for current/external-state
   questions when relevant tools exist.
2. Extend workspace persona rendering to show, for each injected persona file:
   - where the currently injected content resolved from
   - where on-disk updates should be written for the active writable overlay
3. Teach `bash` capability classification that shell-local control fragments do
   not imply writes by themselves:
   - `cd ... && ls` => `read`
   - `cd ... && pwd` => `read`
   - `cd ... && curl ...` => `network`
   - `cd ... && rm ...` => `write`

#### Tests

1. Prompt assembly tests for grounded persona paths.
2. `bash` classification unit tests for `cd` + read/network/write chains.
3. Profile/self-eval fixture updates to validate the prompt/profile behavior.

### PR2: Pre-Emit Capability-Contradiction Recovery

#### Scope

1. Convert response guardrails for capability contradictions from warning-only to
   pre-emit recovery.
2. Keep the final emitted assistant text free of “cannot access current data”
   contradictions when relevant tools exist.
3. Add deterministic runtime and harness coverage.

#### Primary Files

1. `crates/runtime/src/runtime/response_guardrails.rs`
2. `crates/runtime/src/runtime/turn_executor.rs`
3. `docs/skills_and_tools.md`
4. `docs/harness/README.md`
5. `docs/harness/scenarios/...` or runtime tests, depending on runner support

#### Expected Code Changes

1. Replace the current warning-only contradiction path with a guarded retry path
   before output emission.
2. Preserve streaming/non-streaming coherence by only surfacing accepted text to
   users.
3. Emit structured audit/warning events describing the contradiction and
   recovery decision instead of emitting the contradictory user-visible text.

#### Tests

1. Runtime tests should assert:
   - contradictory draft is not emitted to the user
   - recovery happens at most once per contradiction
   - tool-call drafts still bypass the contradiction retry path
2. Harness/blocking scenario should assert that current-data requests with
   available network tools do not produce a final “I cannot access current
   data” answer.

### PR3: Durable Payload Hygiene

#### Scope

1. Separate live tool payloads from durable rollout payloads.
2. Redact sensitive headers/tokens/cookies before persistence.
3. Preserve auditable digests, previews, and dedupe semantics.
4. Update rollout/architecture wording to match durable redaction behavior.

#### Primary Files

1. `crates/runtime/src/runtime/tool_orchestrator.rs`
2. `crates/runtime/src/session.rs`
3. `crates/runtime/src/rollout.rs`
4. `docs/architecture.md`
5. `docs/spec/kernel_contract.md`
6. `docs/skills_and_tools.md`

#### Expected Code Changes

1. Introduce a single persistence transform for tool results before they are
   written to rollout/effect durable surfaces.
2. Redact sensitive fields from common network payload shapes:
   - `authorization`
   - `proxy-authorization`
   - `cookie`
   - `set-cookie`
   - API key / bearer-token style fields
3. Keep enough durable information for audit and dedupe:
   - status / exit code
   - a safe result preview
   - digest of the canonical durable payload
   - redaction summary metadata

#### Tests

1. Rollout persistence tests proving raw secret-bearing headers are not stored.
2. Recovery/dedupe tests proving replay still works with durable payloads.
3. Session/tool-message tests proving live payload use inside the same turn is
   preserved where required.

## Detailed File-by-File Notes

### `crates/runtime/src/prompts/workspace.rs`

Add a richer persona context rendering format. The existing content injection
should stay, but each file section should also expose:

1. `Resolved from: <path>`
2. `Write updates to: <path>` when the writable overlay differs

This is grounded runtime context, not a new public contract.

### `crates/tools/src/lib.rs`

Refine `classify_bash_fragment()` so shell-local control commands such as `cd`
delegate capability to the following command fragment rather than defaulting to
`write`.

Keep the implementation intentionally conservative:

1. fix the clearly wrong cases
2. do not attempt broad shell semantic inference

### `crates/runtime/src/runtime/response_guardrails.rs`

Promote capability-contradiction handling to an actionable recovery decision.
The guardrail should return enough information for the executor to ask the model
for one corrected retry before emission.

### `crates/runtime/src/runtime/turn_executor.rs`

Move contradiction handling into the generation loop before text deltas are
emitted or persisted. The current “disabled for parity” warning path should be
replaced by one consistent accepted-output path.

### `crates/runtime/src/runtime/tool_orchestrator.rs`

Introduce a helper that derives:

1. `live_payload`
2. `durable_payload`
3. optional redaction metadata / digest

Use the live payload for immediate in-turn reasoning when needed, but persist
the durable payload.

### `crates/runtime/src/session.rs`

Ensure tool-message persistence respects the durable payload policy instead of
blindly persisting every full payload shape.

### `crates/runtime/src/rollout.rs`

Keep rollout schema stable where possible, but allow durable records to carry a
redaction summary if needed. Prefer additive fields over format churn.

## Harness Plan

### Self-Eval / Prompt Profile

Use the existing baseline/candidate profile split to measure verification-first
prompt behavior before and after the candidate prompt changes.

Target additions:

1. a candidate profile that includes stronger verification-first guidance
2. scenario assertions or metrics focused on avoiding unverified limitation
   claims

### Blocking Runtime Regression

Add a deterministic regression for:

1. current-data question
2. network-capable tool available
3. model first draft incorrectly claims it cannot access current data
4. runtime recovers before final emission

### Durable Payload Hygiene Regression

Add rollout assertions ensuring secret-bearing network headers are absent from
persisted rollout/effect payloads while safe previews remain available.

## Docs Update Plan

Update existing docs instead of creating a new top-level spec:

1. `docs/skills_and_tools.md`
   - verification-first product behavior
   - contradiction recovery semantics
   - durable payload redaction note
2. `docs/architecture.md`
   - rollout phrasing: auditable durable record, not raw secret dump
3. `docs/spec/kernel_contract.md`
   - rollout as audit chain, possibly redacted on durable surfaces
4. `docs/harness/README.md`
   - new verification-first / contradiction-recovery regression scenario

## Deferred Follow-Ups

These are intentionally out of scope for the first three PRs:

1. Add `ToolCapability::Unknown` to the public protocol and policy engine.
2. Introduce a first-class memory retrieval tool contract.
3. Add generalized shell semantic analysis beyond the narrow misclassified
   cases above.
4. Add public API fields that expose live vs durable payload separation
   directly, unless PR3 proves this is necessary.

## Exit Criteria

1. Current/external-state questions no longer produce final unverified “no
   access” answers when relevant tools exist.
2. Persona file verification/persistence no longer relies on guessed relative
   paths.
3. Plainly read-only shell-control command chains are not mislabeled as
   `write`.
4. Durable rollout records no longer persist raw cookie/authorization-style
   fields.
5. Harness and deterministic tests cover all of the above.
