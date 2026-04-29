## ADDED Requirements

### Requirement: Skill-Authored Semantic Memory Surfaces
Memory and handoff surfaces SHOULD prefer semantic summaries authored by the active agent through the memory skill or another explicit agent-visible memory workflow.

#### Scenario: Agent wraps significant work
- **WHEN** a turn or session changes durable project state, decisions, constraints, open loops, or next steps
- **THEN** the memory skill instructs the agent to write a bounded semantic continuation summary using ordinary governed tools rather than relying on a hidden runtime summarization call

#### Scenario: Runtime refreshes memory surfaces
- **WHEN** runtime refreshes generated memory surfaces at turn end
- **THEN** it SHALL NOT initiate an extra hidden model request solely to summarize memory state

### Requirement: Coherent Fallback Memory Truncation
Generated fallback memory surfaces SHALL truncate text on coherent Markdown or line boundaries and mark omissions explicitly.

#### Scenario: Long Markdown assistant output is rendered by fallback
- **WHEN** memory rendering includes a long Markdown assistant message containing headings, bullets, or code fences
- **THEN** the rendered memory surface does not cut a heading, bullet, or code fence mid-fragment

#### Scenario: Memory text is omitted
- **WHEN** memory rendering omits text due to size limits
- **THEN** the rendered surface includes an explicit truncation marker and a source session or rollout reference when available

### Requirement: Terminal Plan State Refresh
Agent-authored or generated fallback memory, handoff, session-summary, and daily-note surfaces SHALL refresh after terminal turn state is known.

#### Scenario: Turn completes successfully
- **WHEN** a turn completes successfully after plan items were in progress
- **THEN** the generated surfaces do not retain stale `in_progress` plan items unless the final plan snapshot still marks them as open

#### Scenario: Turn ends without assistant text
- **WHEN** a turn ends through tool-only or cancellation paths
- **THEN** memory surfaces render an accurate terminal state message instead of implying active work is still ongoing

### Requirement: Rollout Remains Source Of Truth
Memory surfaces SHALL stay compact continuation aids and point to rollouts when detail is omitted.

#### Scenario: Detail exceeds memory budget
- **WHEN** recent conversation detail exceeds the memory surface budget
- **THEN** the memory surface keeps a coherent summary and identifies where to inspect the raw rollout for full detail
