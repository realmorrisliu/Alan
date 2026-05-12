## ADDED Requirements

### Requirement: Memory Surfaces Reference Proactive Write Provenance
Memory, handoff, session-summary, and daily-note surfaces SHALL remain compact
continuation aids and SHALL reference proactive memory write provenance when
stable-memory detail is omitted.

#### Scenario: Stable write detail is omitted from recall
- **WHEN** a recall or handoff surface includes a stable fact whose detailed
  provenance is stored in the write ledger
- **THEN** the surface includes the bounded fact and SHALL preserve a source
  reference that lets the ledger or rollout be inspected

#### Scenario: Surface does not become ledger
- **WHEN** runtime renders a memory surface after proactive memory writes
- **THEN** the surface does not duplicate the full write ledger content inside
  prompt-facing memory text

### Requirement: Reverted Memory Is Excluded From Prompt Surfaces
Memory, recall, handoff, session-summary, and daily-note surfaces SHALL exclude
stable memory content from reverted memory writes.

#### Scenario: Reverted content remains marked in target file
- **WHEN** a stable memory target contains a machine-readable reverted block for
  a previously inserted write
- **THEN** prompt-facing memory renderers exclude that reverted block from
  generated memory surfaces

#### Scenario: Reverted content was removed
- **WHEN** a reverted memory write was removed from the stable memory target
- **THEN** prompt-facing memory renderers do not reintroduce the removed content
  from the ledger

## MODIFIED Requirements

### Requirement: Skill-Authored Semantic Memory Surfaces
Memory and handoff surfaces SHALL prefer bounded semantic summaries authored by
the active agent through the memory skill or another explicit agent-visible
memory workflow. When that workflow causes stable, staged, inbox, daily-note, or
ledger memory writes after the runtime memory writer is enabled, the durable
write SHALL go through the runtime writer or its compatibility bridge rather
than direct file mutation by ordinary tools.

#### Scenario: Agent wraps significant work
- **WHEN** a turn or session changes durable project state, decisions,
  constraints, open loops, or next steps
- **AND** the runtime memory writer is enabled
- **THEN** the memory skill instructs the agent to produce a bounded semantic
  continuation summary through an explicit agent-visible memory workflow
- **AND** any stable, staged, inbox, daily-note, or ledger memory write caused
  by that workflow goes through the runtime writer or compatibility bridge

#### Scenario: Runtime refreshes memory surfaces
- **WHEN** runtime refreshes generated memory surfaces at turn end
- **THEN** it SHALL NOT initiate an extra hidden model request solely to
  summarize memory state
