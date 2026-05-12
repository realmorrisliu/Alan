## ADDED Requirements

### Requirement: Proactive Memory Write Planning
Alan SHALL run bounded runtime-owned memory write planning for eligible turns,
session finalization, and consolidation passes, and SHALL allow durable facts
from direct user statements, repeated behavior, and external or repository
evidence to become candidate stable memory writes.

#### Scenario: Direct stable user statement is eligible
- **WHEN** a user directly states a stable identity, preference, constraint, or
  workspace rule
- **THEN** the write planner can produce a candidate stable memory write with
  direct-statement evidence

#### Scenario: Repeated behavior is eligible
- **WHEN** Alan observes the same durable user preference or workspace operating
  pattern across multiple source turns or sessions
- **THEN** the write planner can produce a candidate stable memory write with
  repeated-behavior evidence

#### Scenario: External evidence is eligible
- **WHEN** Alan inspects files, issue or PR threads, command output, or
  user-authorized external sources that directly support a durable fact
- **THEN** the write planner can produce a candidate stable memory write with
  source references for that evidence

### Requirement: Runtime-Validated Memory Mutations
Alan SHALL validate and canonicalize every memory write plan before mutating
stable memory files. The model SHALL NOT directly mutate stable memory files as
the authority for proactive memory writes.

#### Scenario: Invalid target is rejected
- **WHEN** a write plan names a target outside the allowed memory layout
- **THEN** runtime rejects the candidate before any stable memory file is
  mutated

#### Scenario: Over-broad candidate is downgraded
- **WHEN** a candidate has useful evidence but insufficient confidence for
  stable memory
- **THEN** runtime records or keeps it as staged memory rather than promoting it
  into stable memory

#### Scenario: Duplicate stable fact is deduped
- **WHEN** a write plan repeats an existing stable memory observation without
  materially updating it
- **THEN** runtime avoids adding a duplicate stable memory entry

### Requirement: Durable Memory Write Ledger
Alan SHALL record every stable memory mutation in a durable ledger entry with a
stable `memory_write_id`, target, provenance, confidence, rationale, source
session or turn metadata, and revert status.

#### Scenario: Stable write creates ledger entry
- **WHEN** runtime promotes a candidate into `USER.md`, `MEMORY.md`, or a topic
  page
- **THEN** it records a ledger entry that identifies the inserted stable memory
  content and its source evidence

#### Scenario: Ledger omits hidden reasoning
- **WHEN** runtime records a ledger entry produced by model-mediated planning
- **THEN** the ledger stores observation, evidence, confidence, and rationale
  without storing hidden chain-of-thought or provider-private reasoning content

#### Scenario: Legacy memory lacks reversible ledger
- **WHEN** a stable memory entry predates the ledger
- **THEN** Alan treats it as legacy memory and does not claim it is
  automatically reversible

#### Scenario: Ledger is one file per write
- **WHEN** runtime records a stable memory mutation
- **THEN** it writes a dedicated Markdown ledger file for that
  `memory_write_id` under the memory ledger directory

### Requirement: Reviewable Evidence Provenance
Alan SHALL persist enough provenance for each stable memory write to let a user
review why the fact was written without storing large raw artifacts or hidden
reasoning.

#### Scenario: File evidence is recorded
- **WHEN** a stable memory write is based on repository or local file evidence
- **THEN** the ledger records the source kind, path, and a bounded line range,
  excerpt, or content hash for the evidence

#### Scenario: Command evidence is recorded
- **WHEN** a stable memory write is based on command output
- **THEN** the ledger records the command identity, observed-at time, and a
  bounded non-secret excerpt or summary of the output

#### Scenario: External evidence is recorded
- **WHEN** a stable memory write is based on a URL, issue, PR, or other external
  source
- **THEN** the ledger records the source locator, observed-at time, and a
  bounded excerpt or summary sufficient for later review

### Requirement: Sensitive Data Memory Guardrail
Alan SHALL reject or redact memory candidates and evidence that contain
secret-like or credential-like material before any stable, staged, inbox,
daily-note, consolidation, or ledger persistence.

#### Scenario: Secret-like candidate is rejected
- **WHEN** a memory candidate observation contains an API key, token, password,
  private credential, or secret-like value
- **THEN** runtime rejects the memory write or rewrites it into a redacted
  non-secret observation before durable persistence

#### Scenario: Secret-like staged candidate is rejected
- **WHEN** a memory candidate or its evidence contains secret-like material
- **AND** the candidate would otherwise be staged for inbox, daily-note, or
  consolidation review rather than promoted to stable memory
- **THEN** runtime rejects the staged write or rewrites it into redacted
  non-secret content before writing any durable staged memory

#### Scenario: Secret-like evidence is redacted
- **WHEN** evidence for a memory write contains secret-like material
- **THEN** the ledger omits or redacts the secret-like material while preserving
  enough non-secret provenance to explain the write

### Requirement: Recent Memory Write Inspection
Alan SHALL expose low-disturbance recent-write inspection surfaces for stable
memory writes without interrupting normal agent turns.

#### Scenario: Recent writes are listed
- **WHEN** a user asks for recent memory writes through CLI or daemon API for a
  workspace or session
- **THEN** Alan returns bounded write metadata including id, timestamp, target,
  observation, confidence, and revert status

#### Scenario: Write detail is shown
- **WHEN** a user requests a single memory write by id for a workspace or
  session
- **THEN** Alan returns the write detail, provenance, target location, and
  revert eligibility

### Requirement: Reversible Stable Memory Writes
Alan SHALL support precise revert for stable memory writes when the ledger entry
still matches the target memory content, and SHALL fail safely when precise
revert cannot be proven.

#### Scenario: Revert succeeds
- **WHEN** a user reverts a memory write whose anchored content still matches
  the target memory file
- **THEN** Alan removes the inserted stable memory content or marks it with a
  machine-readable reverted state that prompt-facing memory surfaces must filter
- **AND** Alan updates the ledger revert status

#### Scenario: Reverted write is not prompt-visible
- **WHEN** a memory write has been reverted successfully
- **THEN** its inserted stable memory content is not eligible for future
  prompt-facing recall, handoff, session-summary, or daily-note surfaces

#### Scenario: Revert cannot be proven
- **WHEN** the target memory file has changed so that Alan cannot identify the
  inserted content safely
- **THEN** Alan leaves the file unchanged and marks the write as requiring
  manual resolution

### Requirement: Ambiguous Memory Consolidation
Alan SHALL stage ambiguous, conflicting, or cross-session memory observations
for consolidation instead of forcing immediate stable-memory promotion.

#### Scenario: Conflicting observation is staged
- **WHEN** a candidate conflicts with existing stable memory
- **THEN** Alan records the observation for consolidation and does not silently
  overwrite the stable memory entry

#### Scenario: Consolidation promotes resolved observation
- **WHEN** a consolidation pass resolves staged observations into a stable fact
- **THEN** Alan writes the stable memory through the same validated mutation and
  ledger path as turn-end promotion
