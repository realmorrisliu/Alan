## Why

Alan can already delegate work, persist rollouts, gate tools, and refresh memory
surfaces, but those pieces do not yet form a closed product contract for a
long-running agent. Recent session analysis showed that capability mismatch,
unreadable child artifacts, truncated evidence, invisible delegated progress,
stale yielded states, and low-signal memory goals can make correct work feel
opaque or unreliable.

## What Changes

- Add capability-aware delegation so Alan matches task requirements against
  available tools, skills, child targets, and escalation paths before launching
  delegated work.
- Add an auditable evidence pipeline so tool outputs, delegated outputs, and
  answer-supporting artifacts are durable, inspectable, and provenance-linked
  even when prompt-facing payloads are truncated.
- Add a human-visible run lifecycle so parent turns, child runs, approvals,
  resumes, progress, and terminal states are exposed as coherent state
  transitions to daemon clients and UI surfaces.
- Require delegated output references to be runtime-readable under parent
  authorization instead of exposing raw file paths that the parent workspace may
  not be allowed to read.
- Require memory surfaces to preserve substantive task goals and avoid replacing
  session intent with short ambiguous replies, runtime control messages, or
  approval controls.
- Preserve workspace isolation and governance: the runtime may expose authorized
  artifacts and lifecycle metadata without turning cross-workspace child files
  into ordinary parent filesystem reads.

## Capabilities

### New Capabilities

- `agent-capability-routing`: Owns task requirement extraction, capability
  matching, delegation eligibility, fallback/recovery decisions, and visible
  capability-mismatch reporting.
- `runtime-evidence-provenance`: Owns durable evidence artifacts, evidence
  references, truncation/provenance metadata, and answer evidence summaries for
  tool and child outputs.
- `human-visible-run-lifecycle`: Owns user-visible session/run state transitions,
  approval/resume state recovery, delegated-work progress events, and client
  timeline semantics.

### Modified Capabilities

- `child-run-lifecycle`: Child runs must emit parent-visible start/progress/
  completion lifecycle events and not only update registry records.
- `delegated-result-handoff`: Delegated `output_ref` values must resolve through
  an authorized runtime artifact reader instead of requiring parent filesystem
  access to a child workspace path.
- `daemon-api-contract`: Daemon API metadata must cover evidence-artifact and
  lifecycle/progress surfaces introduced by this change.
- `runtime-memory-surfaces`: Current-goal derivation must use substantive user
  intent and plan state, not the latest low-information user message.

## Impact

- Affected runtime modules: child agent orchestration, virtual tool execution,
  tool-result persistence, rollout recording, tool-policy approval replay,
  turn/run state bridging, and memory surface rendering.
- Affected daemon/client areas: session/run state DTOs, child-run surfaces,
  evidence-artifact read APIs, event timelines, and UI status labels.
- Affected built-in skills: delegated skill instructions should reflect typed
  capability requirements, capability mismatch recovery, and evidence
  inspection semantics.
- Affected storage: rollouts or adjacent artifact storage must preserve full
  answer-supporting evidence without leaking secrets or bypassing workspace
  authorization.
- Affected tests: capability routing, child output artifact reads, durable
  evidence preservation, approval resume status transitions, delegated progress
  visibility, and memory goal salience.
