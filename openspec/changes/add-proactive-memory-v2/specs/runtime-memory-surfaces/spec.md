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
