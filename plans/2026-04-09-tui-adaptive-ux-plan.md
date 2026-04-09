# Alan TUI Adaptive UX Plan (2026-04-09)

> Status: active implementation plan for the TUI adaptive-interaction line.

## Why This Plan Exists

The TUI adaptive-yield work is no longer at a blank-slate stage.

After `#74` and `#88`, the local codebase already includes:

1. yield-type renderers under `clients/tui/src/adaptive-surfaces/`,
2. a confirmation card,
3. structured input forms,
4. schema-driven forms for `dynamic_tool` and `custom`,
5. protocol support for richer structured kinds and presentation hints.

What is still missing is not a brand-new protocol direction. The gap is a
product-layer interaction contract and a sequenced plan for finishing the TUI
as a coherent operator console.

Use `docs/spec/alan_tui_ui_ux.md` as the target behavior contract for the
remaining work.

## Current Baseline

### Already Present In The Local Codebase

1. Adaptive surface registry and dedicated renderer modules.
2. Confirmation, structured-input, and schema-driven yield rendering.
3. Keyboard-first yield handling with slash-command fallbacks.
4. Timeline support for protocol yields and legacy `plan_updated` events.

### Still Missing Or Incomplete

1. Persistent plan/progress visibility outside the timeline.
2. A compact runtime activity / recoverable-error summary surface.
3. Full confirmation-card alignment with protocol defaults and hints.
4. Better separation between top-level TUI orchestration and adaptive surface
   state/control logic.
5. A clear issue-to-implementation mapping now that some open issue acceptance
   criteria are partially satisfied by current code.

## Recommendation

Yes: this line should have both:

1. a product-facing UX contract,
2. an implementation plan.

No: it does not currently need another protocol-first spec before continuing.

The protocol baseline is already far enough along that the next bottleneck is
TUI product coherence, not contract invention.

## Open Issue Recalibration

The open issue set should be read against the current codebase, not against the
pre-`#88` baseline.

### `#87` Extract TUI yield rendering into dedicated adaptive surface modules

Status in the local tree:

1. materially started,
2. not fully finished.

What remains:

1. move more yield-specific orchestration and state assembly out of
   `clients/tui/src/index.tsx`,
2. give adaptive surfaces a cleaner controller boundary,
3. avoid letting top-level app code become the long-term home for summary-state
   logic.

### `#89` Build adaptive confirmation cards in the TUI

Status in the local tree:

1. baseline card rendering exists,
2. acceptance appears partially satisfied,
3. final polish and protocol-alignment still remain.

What remains:

1. respect `default_option`,
2. honor dangerous/default semantics more intentionally,
3. improve scanability for common structured detail payloads.

### `#90` Extend structured input with richer widgets and presentation hints

Status in the local tree:

1. protocol support is ahead of the original issue wording,
2. TUI rendering parity is not complete.

What remains:

1. make current semantic kinds feel intentional in the UI,
2. decide which presentation hints are worth honoring now,
3. avoid reopening protocol work unless the TUI hits a real contract gap.

### `#91` Add an active tool and recoverable-error HUD to the TUI

Status:

1. not yet properly shipped,
2. currently covered only indirectly by header text and timeline rows.

### `#92` Add a persistent plan and progress panel to the TUI

Status:

1. not yet shipped,
2. currently represented only as transient `plan_updated` timeline events.

## Delivery Order

Recommended sequence from the current baseline:

1. lock the UX contract and finish the extraction boundary,
2. add persistent plan visibility,
3. add runtime activity / recoverable-error summary,
4. close the confirmation-card gap against the contract,
5. finish richer semantic widget and hint support.

This differs slightly from the original `#86` note because the local tree has
already moved partway through the old `#87` and `#89` scope.

## Implementation Slices

### Slice 0: Contract Lock-In

Scope:

1. land `docs/spec/alan_tui_ui_ux.md`,
2. use it as the reference for remaining TUI PRs,
3. update issue descriptions or comments later if needed so the backlog matches
   the real baseline.

Exit criteria:

1. the team has one normative TUI UX document,
2. future PRs can be reviewed against a stable interaction model.

### Slice 1: Finish Adaptive Controller Extraction (`#87`)

Scope:

1. shrink `clients/tui/src/index.tsx` responsibilities,
2. extract adaptive-surface context assembly and controller helpers,
3. prepare clean seams for plan and runtime-summary surfaces.

Likely code direction:

1. keep renderer modules under `clients/tui/src/adaptive-surfaces/`,
2. add a controller or view-model layer for pending-yield state,
3. keep slash-command parsing separate from surface-local keyboard behavior.

Exit criteria:

1. `index.tsx` no longer owns most yield-specific coordination logic,
2. surface rendering and surface control logic are easier to extend safely,
3. tests cover controller/helper behavior rather than only entry-file behavior.

### Slice 2: Persistent Plan Surface (`#92`)

Scope:

1. derive current plan state from incoming events,
2. render a compact persistent plan panel,
3. keep `plan_updated` in the timeline as history.

Rules:

1. no new protocol work unless a real missing field appears,
2. prefer normalization helpers and tests over inline event scanning in the
   render path.

Exit criteria:

1. plan state remains visible while the agent continues streaming,
2. the active step is obvious without scrolling,
3. the panel disappears or collapses gracefully when no plan is active.

### Slice 3: Runtime Activity and Recovery Surface (`#91`)

Scope:

1. derive active tool state from tool lifecycle events,
2. surface waiting state such as yielded, retrying, or idle,
3. show the most relevant recoverable error with actionable guidance.

Rules:

1. this is a compact summary, not a second timeline,
2. it should complement the header rather than replace it.

Exit criteria:

1. the user can identify what the runtime is doing now at a glance,
2. recoverable failures are actionable without log archaeology,
3. the panel remains visually subordinate to a pending action surface.

### Slice 4: Confirmation Alignment and Polish (`#89`)

Scope:

1. align the current confirmation card with the UX contract,
2. improve default action focus and dangerous-state rendering,
3. polish structured detail formatting where it improves scanability.

Important note:

1. if the current confirmation-card baseline is not yet on the target branch,
   ship that baseline first;
2. if it is already effectively landed, treat this slice as a closeout/polish
   pass rather than a net-new feature.

Exit criteria:

1. confirmation flows feel like a first-class panel rather than a log alias,
2. slash commands remain intact as fallbacks,
3. protocol default semantics are reflected in the UI.

### Slice 5: Richer Semantic Widgets and Hint Support (`#90`)

Scope:

1. finish rendering support for already-defined semantic kinds,
2. selectively honor useful presentation hints,
3. decide whether any remaining gaps actually require protocol changes.

Suggested priority inside this slice:

1. boolean and numeric fields feel intentional,
2. `dangerous`, `toggle`, and `multiline` hints get sensible TUI treatment,
3. searchable or large-option-set behavior is deferred unless it has a clean
   keyboard-first implementation.

Exit criteria:

1. at least one meaningful hint-driven improvement lands end-to-end,
2. semantic kinds remain the source of truth,
3. unsupported hints degrade safely without confusing behavior.

## Review Heuristics

Each TUI PR on this line should be reviewed against these questions:

1. Does this reduce or increase reliance on slash-command memorization?
2. Does it make the operator's current-state scan faster?
3. Does it preserve the timeline as history rather than overload it as the only
   state surface?
4. Does it keep keyboard interaction primary?
5. Does it preserve a raw fallback for unknown payloads?

## Suggested Follow-Up After Docs Land

1. Implement Slice 1 first and keep it narrowly mechanical.
2. Then land Slice 2 and Slice 3 as small vertical additions.
3. Reassess `#89` and `#90` after the plan and activity surfaces exist, because
   the remaining UX pressure will be easier to see in context.
