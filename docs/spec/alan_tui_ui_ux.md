# Alan TUI UI / UX Contract

> Status: VNext product contract for `alan-tui`.

## Purpose

Define the normative operator experience for `alan-tui` so the TUI keeps a
clear, keyboard-first interaction model as adaptive yields, plan updates, and
runtime status surfaces grow.

This document exists to stop the TUI from drifting into a transport-shaped
mixture of:

1. raw event log inspection,
2. slash-command memorization,
3. one-off yield-specific local UI experiments.

It complements:

1. `docs/spec/interaction_inbox_contract.md` for input semantics,
2. `docs/skills_and_tools.md` for runtime-facing tool and yield behavior,
3. `crates/protocol/src/adaptive.rs` for protocol payload shapes.

When the current TUI implementation conflicts with this document, the TUI
should be refactored toward this contract.

## Product Thesis

`alan-tui` is a terminal-native operator console for long-running agent work.

It must feel:

1. keyboard-first,
2. calm and compact,
3. easy to scan under pressure,
4. capable of guided interaction without losing raw terminal escape hatches.

It must not feel like:

1. a chat app with extra logs,
2. a JSON inspector with slash commands,
3. a dashboard that duplicates every event in three places.

## Primary User Jobs

The TUI must optimize for these jobs in this order:

1. understand what the agent is doing now,
2. resolve the next required user action quickly,
3. keep enough history visible to trust what happened,
4. steer or follow up without losing terminal flow.

## Non-Goals

This contract does not require:

1. mouse-first interaction,
2. hiding slash commands or raw resume paths,
3. a separate modal command palette,
4. a dashboard-like permanent multi-column layout,
5. protocol-specific heuristics for unsupported future yield payloads.

## Core Layout

The TUI layout is a vertical operator stack:

```text
Header
Action Surface?        (only when user-facing action is pending)
Summary Surfaces?      (plan and runtime status, compact and additive)
Timeline               (primary transcript and event history)
Hint Line
Composer
```

Rules:

1. The timeline remains the primary historical surface.
2. The composer remains the primary text entry point.
3. A pending action surface outranks every summary surface.
4. Summary surfaces must compress state, not replay the whole timeline.
5. The screen must remain usable when only header + timeline + composer are
   visible.

## Information Hierarchy

At a glance, a user should read the TUI in this order:

1. whether the runtime needs them right now,
2. what the runtime is doing or waiting on,
3. what the current plan/progress state is,
4. only then the full timeline history.

This implies:

1. pending yields get the most prominent non-header surface,
2. active tool / retry / recoverable-error state must be scannable without
   reading the whole log,
3. plan state must be visible outside transient `plan_updated` events,
4. the timeline stays authoritative for chronology and detail.

## Header Contract

The header is a compact status strip, not a hero panel.

It should show:

1. product identity,
2. connection or daemon state,
3. current session identity,
4. current runtime status such as `running`, `yielded`, or `ready`,
5. pending action summary when applicable.

It should not:

1. restate long help text,
2. list every available command,
3. compete visually with the action surface or timeline.

IDs such as `session_id` or `request_id` may appear in reduced emphasis, but
human-readable state must dominate.

## Action Surface Contract

An action surface is the focused panel for a pending yield.

Rules:

1. There is at most one active action surface at a time.
2. Known first-class yield kinds must render as guided UI, not raw JSON.
3. Slash commands remain available as a fallback, not as the primary path.
4. Unknown or unsupported payload fragments must degrade safely to `/resume`.

### Confirmation Surface

The confirmation surface must present:

1. a short summary first,
2. structured details second,
3. available actions third,
4. local keyboard help last.

Rules:

1. The default focus should respect protocol intent.
   Prefer `default_option` when provided; otherwise prefer `approve` when
   present; otherwise use the first option.
2. Detail rendering should favor scan-friendly rows for common fields such as
   command, path, tool, diff, policy, and replay metadata.
3. Dangerous actions should be visually distinct when the payload semantics or
   presentation hints justify it.
4. Keyboard resolution must be possible without leaving the panel.

Required parity:

1. keyboard-first resolution,
2. slash fallback via `/approve`, `/reject`, `/modify`,
3. no raw JSON requirement for normal confirmation flows.

### Structured Input Surface

Structured input must be a first-class guided form.

It should present:

1. title and prompt,
2. current question position,
3. compact previews of every question's current answer,
4. the active question's controls and validation state,
5. a manual fallback example.

Rules:

1. The active question must be obvious.
2. Validation must block submit clearly and locally.
3. Text entry should flow through the composer when the active field is text-like.
4. Single-select and multi-select questions must not require manual JSON.
5. Slash fallback via `/answer`, `/answers`, and raw `/resume` remains valid.

### Dynamic Tool and Custom Surfaces

`dynamic_tool` and `custom` yields should render an adaptive form when the
payload exposes a stable form contract.

Rules:

1. Schema-driven rendering is preferred when the payload is explicit enough.
2. Tool or custom context should stay visible near the form.
3. Unsupported shapes must degrade cleanly to `/resume <json>`.
4. The presence of a schema must never remove the raw fallback path.

## Summary Surfaces Contract

Summary surfaces are persistent state views, not transient event rows.

They sit between the action surface and the timeline when useful.

### Plan Surface

The plan surface should show:

1. current ordered steps,
2. step status (`pending`, `in_progress`, `completed`),
3. the currently active step at a glance,
4. optional explanation text when the plan changes meaningfully.

Rules:

1. It must persist across ongoing streaming output.
2. It must not require the user to scroll the timeline to recover the current
   plan.
3. It should degrade cleanly when no plan exists.
4. It should stay compact enough to avoid pushing the timeline off-screen.

The timeline should still keep `plan_updated` events for historical audit.

### Runtime Activity and Recovery Surface

The runtime status surface should summarize what is happening now, not the full
history of what happened.

It should show:

1. currently running or most recent tool,
2. whether the runtime is waiting on a yield, tool, retry path, or idle state,
3. the most relevant recoverable error when one exists,
4. the next reasonable user action when recovery is possible.

Rules:

1. It must be compact and secondary to the action surface.
2. It must not duplicate the full timeline verbatim.
3. Recoverable errors must appear as actionable guidance, not just colored text.

## Timeline Contract

The timeline is the authoritative chronological surface.

It should:

1. preserve turn, tool, yield, warning, error, and assistant output history,
2. summarize protocol events in human-readable language,
3. keep adaptive surfaces and summary surfaces additive rather than replacing
   it.

It should not be the only place where the user can answer:

1. what is the plan,
2. what tool is active,
3. what is waiting on me.

## Hint Line Contract

The hint line is contextual, terse, and local to the current state.

Rules:

1. It should advertise only the most relevant current controls.
2. It should change when the active action surface changes.
3. It should not repeat the full `/help` output.

## Composer Contract

The composer is the single persistent text-entry control.

Rules:

1. It remains present in every normal state.
2. Its label and placeholder should adapt to the active context.
3. When the active action surface needs typed text, the composer becomes the
   input path for that field.
4. When the active action surface is primarily selection-based, the composer may
   remain unfocused until the user types `/`.

The composer must support both:

1. normal user turns,
2. slash command escape hatches.

## Keyboard and Slash Command Model

The interaction model is:

1. keyboard-first for known adaptive surfaces,
2. slash-command fallback for explicit control,
3. raw `/resume <json>` fallback for unknown or forward-compatible payloads.

### Global Commands

Global slash commands cover session and runtime control:

1. session lifecycle and connection,
2. auth,
3. interrupt,
4. compact,
5. rollback,
6. help and exit.

### Yield Aliases

Yield aliases are ergonomic wrappers over known payload shapes:

1. `/approve`, `/reject`, `/modify`,
2. `/answer`, `/answers`,
3. `/resume <json>`.

Rules:

1. Every keyboard-driven resolution path should have a slash equivalent.
2. The slash model should stay small and semantically stable.
3. New slash commands should be added only when they materially reduce operator
   friction.

## Adaptive Semantics and Presentation Hints

Semantic field kind remains primary. Presentation hints are secondary.

Rules:

1. Clients should first honor semantic kinds such as `boolean`,
   `single_select`, `multi_select`, `number`, and `integer`.
2. Presentation hints such as `toggle`, `searchable`, `multiline`, `compact`,
   and `dangerous` may refine rendering when practical.
3. If a hint cannot be rendered faithfully in the TUI, the client should
   degrade gracefully without changing the underlying semantics.
4. Hints must not force protocol-specific frontend heuristics to become the
   source of truth.

## Copy and Tone

The TUI's copy should be:

1. concise,
2. operator-facing,
3. explicit about next actions.

It should avoid:

1. long explanatory paragraphs in the steady state,
2. redundant restatement of visible information,
3. raw protocol jargon as the primary UI language when a human-readable label
   exists.

## Acceptance Criteria

1. The TUI remains simple and keyboard-native under long-running execution.
2. Known adaptive surfaces are operable without forcing slash-command
   memorization.
3. Slash-command escape hatches remain available for explicit and
   forward-compatible control.
4. Operators can identify pending action, current runtime activity, and plan
   progress without timeline spelunking.
