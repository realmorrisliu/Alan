## ADDED Requirements

### Requirement: Coding steward contracts live in OpenSpec
alan SHALL specify coding steward orchestration, repo-worker delegation,
verification honesty, behavior-preserving change policy, coding governance, and
coding eval ladders in OpenSpec.

#### Scenario: Coding workflow behavior changes
- **WHEN** a change modifies parent steward responsibilities, repo-scoped child
  worker responsibilities, minimum repo-worker loop behavior, verification
  reporting, delivery summaries, or coding governance boundaries
- **THEN** the requirement is updated in this capability,
  `agent-capability-routing`, `delegated-result-handoff`, `runtime-evidence-provenance`,
  or another active OpenSpec owner

#### Scenario: Repo-worker package layout is described
- **WHEN** docs describe the first-party repo-worker package path, child launch
  root, micro-skills, scripts, evals, or harness entrypoints
- **THEN** the docs point at current package implementation guides and OpenSpec
  capability owners instead of a historical `plans/` file

### Requirement: Coding verification remains evidence-based
alan SHALL distinguish actual verification from planned, skipped, mocked, or
environment-blocked verification in coding steward and repo-worker outputs.

#### Scenario: Worker reports completion
- **WHEN** a repo-worker or parent steward delivers a coding result
- **THEN** the response includes the commands or checks actually run, failures
  or environment blockers, and remaining risk
- **AND** it does not imply product behavior was proven by checks that did not
  execute or only exercised mocks

### Requirement: Coding evals validate steward and worker layers separately
alan SHALL keep repo-worker package validation, coding steward orchestration
validation, package-local evals, and external benchmark adapters separated by
what behavior each layer proves.

#### Scenario: Harness coverage is documented
- **WHEN** docs or fixtures describe repo-worker or coding-steward scenarios
- **THEN** they remain executable fixture documentation unless they define
  normative behavior, in which case the behavior is captured in OpenSpec
