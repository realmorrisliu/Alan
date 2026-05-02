# delegated-result-handoff Specification

## Purpose
TBD - created by archiving change sub-agent-lifecycle-control. Update Purpose after archive.
## Requirements
### Requirement: Completed Child Output Fidelity
The system SHALL preserve completed delegated child output as full inline text or an inspectable output reference instead of silently replacing it with an unlabeled short preview.

#### Scenario: Child returns short output
- **WHEN** a completed delegated child returns output that fits the parent tape budget
- **THEN** the delegated result includes the full `output_text` and does not mark it as truncated

#### Scenario: Child returns long output
- **WHEN** a completed delegated child returns output that exceeds the parent tape budget
- **THEN** the delegated result includes a bounded preview, an `output_ref` or rollout/session reference for the full text, and truncation metadata that states what was omitted

### Requirement: Delegated Result Shape
The delegated result payload SHALL distinguish summary, preview, full output, child-run reference, structured output, and truncation metadata.

#### Scenario: Completed child result is persisted
- **WHEN** a completed child result is recorded in the parent tape or rollout
- **THEN** the result contains `status`, `summary`, `child_run`, and either `output_text` or `output_ref`

#### Scenario: Summary is shortened
- **WHEN** the result summary is shortened for tape compactness
- **THEN** the result includes `summary_preview` and truncation metadata so the parent can tell the summary was intentionally shortened

#### Scenario: Structured output is large
- **WHEN** structured output exceeds the inline budget
- **THEN** the result preserves critical keys such as `status` and `summary`, includes a structured output reference or bounded structured preview, and records explicit truncation metadata

### Requirement: Failed Child Handoff Metadata
The system SHALL include child-run metadata and latest progress information when a child fails, pauses, is cancelled, is terminated, or times out.

#### Scenario: Child times out
- **WHEN** a delegated child reaches idle timeout
- **THEN** the delegated result includes `error_kind`, `error_message`, child-run reference, rollout path when available, latest heartbeat/progress metadata, and terminal status `timed_out`

#### Scenario: Child is explicitly terminated
- **WHEN** a delegated child is terminated by operator or parent request
- **THEN** the delegated result distinguishes `terminated` from `timed_out` and includes termination actor, reason, and mode when available

