## ADDED Requirements

### Requirement: Memory contracts live in OpenSpec
alan SHALL keep durable memory architecture, pure-text memory layout, recall,
write, compaction coordination, and memory-surface requirements in OpenSpec.

#### Scenario: Memory behavior changes
- **WHEN** a change modifies workspace memory layout, session summaries,
  working memory, handoff files, memory recall, memory write planning,
  promotion, or compaction coordination
- **THEN** the change updates `runtime-memory-surfaces`, `add-proactive-memory-v2`,
  this capability, or another named OpenSpec owner
- **AND** it does not create a new long-form memory contract in `docs/spec/`

#### Scenario: Legacy memory docs remain linked
- **WHEN** `docs/spec/memory_architecture.md` or
  `docs/spec/pure_text_memory_contract.md` is reached during migration
- **THEN** the file points to OpenSpec memory owners as a non-authoritative
  bridge

### Requirement: Memory surfaces remain human-readable and provenance-aware
alan SHALL keep memory and handoff surfaces readable, workspace-scoped, and
explicit about truncation or provenance when they summarize larger runtime
artifacts.

#### Scenario: Memory surface summarizes recent work
- **WHEN** alan writes current-goal, handoff, session, topic, or recall
  material
- **THEN** it preserves substantive user intent and relevant evidence
  references
- **AND** it avoids replacing the goal with low-information control messages

#### Scenario: Memory content is truncated
- **WHEN** memory or handoff content is shortened for readability or prompt
  safety
- **THEN** the truncation is coherent and points to the source rollout,
  evidence, or session context when available
