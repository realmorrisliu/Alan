# Alan Coding Steward Boundary Plan (2026-04-18)

> Status: implemented local consolidation plan for the steward-vs-repo-worker
> boundary track in `#282`.

## Why This Plan Exists

`#10` now frames Alan's coding roadmap as:

1. Alan as the home-root coding steward,
2. repo-scoped coding work executed through fresh child runtimes.

That framing is directionally correct, but the local tree still leaves the
boundary under-specified:

1. the repo previously split the boundary across a top-level steward contract
   and a second repo-worker spec, which duplicated source-of-truth risk.
2. `reference/coding-agent/README.md` needed to become clearly scaffold-local
   rather than a general coding product description.
3. `SpawnSpec` exists in protocol/runtime, but there is no coding-specific
   launch contract covering recommended `cwd`, `workspace_root`, and handle
   bindings.
4. `docs/skills_and_tools.md` documents the narrow delegated-launch default, but
   does not currently separate that runtime-wide default from the richer coding
   handoff shape Alan should use when the steward launches a repo worker.

This plan closes that contract gap before the deeper repo-worker, governance,
and evaluation tracks continue.

## Scope

This batch covers:

1. a new normative coding-boundary spec for Alan as steward plus repo-scoped
   child workers,
2. consolidating the repo-worker contract details into the steward contract and
   keeping only scaffold-local package docs,
3. adjacent doc alignment where readers would otherwise get conflicting
   guidance,
4. issue-tracking cleanup so `#10` and `#282` point at the same execution
   sequence.

This batch does not cover:

1. full repo-worker capability graduation (`#283`),
2. full workspace-aware coding governance (`#284`),
3. external benchmark adapters or steward eval harnesses (`#285`).

## Current Baseline

### Already Present

1. `docs/architecture.md` and `docs/spec/kernel_contract.md` already describe
   `SpawnSpec` as an `exec`-like child-runtime launch contract.
2. `crates/protocol/src/spawn.rs` already exposes the transport-level launch
   shape for `cwd`, `workspace_root`, runtime overrides, and explicit handles.
3. `crates/runtime/src/runtime/child_agents.rs` already implements bounded
   handle binding for `workspace`, `memory`, `plan`,
   `conversation_snapshot`, `tool_results`, and `approval_scope`.
4. `reference/coding-agent/` already provides a runnable scaffold for a
   repo-scoped coding loop.

### Still Missing Or Ambiguous

1. There is no one spec that says Alan itself is the coding steward while
   `reference/coding-agent/` is only the repo-worker layer.
2. There is no coding-specific recommendation for which handles a child coding
   worker should receive by default versus conditionally.
3. There are no canonical parent/child coding examples that show discovery,
   routing, spawn, and result integration as one product story.
4. Adjacent docs can still be read as if delegated-launch defaults and coding
   launch guidance are identical.

## Delivery Order

Recommended sequence:

1. lock this plan in-repo,
2. land the new steward/worker contract spec,
3. reposition the reference coding scaffold docs,
4. align adjacent docs that mention the coding product boundary,
5. tighten `#10` and `#282` tracking language.

## Implementation Slices

### Slice 0: Plan Lock-In

Deliverables:

1. this execution plan,
2. `plans/README.md` update.

Exit criteria:

1. the boundary line has one in-repo execution sequence,
2. later review can audit against explicit slices rather than issue memory.

### Slice 1: Steward vs Repo-Worker Contract

Files:

1. `docs/spec/alan_coding_steward_contract.md`
2. `docs/spec/README.md`
3. optional cross-reference update in `docs/architecture.md`

Required changes:

1. define Alan's coding product boundary as home-root steward plus repo-scoped
   child workers,
2. define the parent steward responsibilities,
3. define the child repo-worker responsibilities,
4. define the coding-specific `SpawnSpec` launch guidance,
5. add canonical workflow examples,
6. separate runtime process ancestry from `AgentRoot` overlay semantics.

Exit criteria:

1. there is one spec readers can use to understand Alan coding without falling
   back to a workspace-first mental model,
2. the coding child launch contract is explicit rather than implied by runtime
   code.

### Slice 2: Consolidate The Repo-Worker Spec

Files:

1. `docs/spec/alan_coding_steward_contract.md`
2. `reference/coding-agent/README.md`

Required changes:

1. merge the still-relevant repo-worker contract details into the steward
   contract,
2. retire the duplicate standalone repo-worker spec,
3. keep only scaffold-local documentation under `reference/coding-agent/`,
4. keep the harness and provider references aligned with the consolidated
   contract.

Exit criteria:

1. `reference/coding-agent/` is clearly understood as a staging scaffold for a
   repo-scoped child worker,
2. there is no second top-level coding spec duplicating the boundary contract,
3. no local doc still implies that Alan coding equals a single-repo shell.

### Slice 3: Adjacent Doc Alignment

Files:

1. `docs/skills_and_tools.md`
2. `docs/spec/provider_auth_contract.md`
3. any small cross-reference updates needed in `docs/architecture.md`

Required changes:

1. clarify that the delegated-skill default launch is intentionally narrow and
   not the whole coding handoff story,
2. clarify that provider/auth work serves the steward/worker coding line rather
   than only a standalone reference coding worker,
3. keep adjacent docs discoverable from the new contract.

Exit criteria:

1. nearby docs do not contradict the new boundary contract,
2. readers can distinguish runtime-wide delegation defaults from coding-specific
   launch guidance.

### Slice 4: Issue Alignment

Deliverables:

1. `#10` milestone cleanup so each milestone maps to a child issue,
2. `#282` wording and tracking alignment if needed.

Exit criteria:

1. `#10` stays the umbrella issue,
2. `#282` is clearly the architecture and boundary track,
3. the order `#282 -> #283 -> #284 -> #285` is explicit in the parent issue.

## Review Heuristics

Review this batch against these questions:

1. Does each doc clearly separate home-root stewardship from repo-local coding
   execution?
2. Does the launch contract explain what is intentionally inherited versus not
   inherited?
3. Does the reference coding scaffold read like a child worker rather than a
   second product?
4. Do adjacent docs avoid reintroducing a workspace-first coding posture by
   implication?

## Suggested Immediate Follow-Up

After this plan lands:

1. use `#283` to stabilize the repo-worker coding loop itself,
2. use `#284` to turn the boundary contract into explicit governance behavior,
3. use `#285` to measure both repo-worker coding and steward orchestration.
