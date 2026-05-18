## ADDED Requirements

### Requirement: macOS shell documentation uses OpenSpec as the contract source
The macOS shell build and verification contract SHALL prevent active macOS
shell documentation from treating `docs/spec/` or task-specific plan files as
the authoritative UI, interaction, build, lifecycle, or runtime contract.

#### Scenario: macOS shell documentation references contracts
- **WHEN** active macOS shell README, architecture, build, install, or
  verification docs reference UI, interaction, lifecycle, runtime, distribution,
  or build/test contracts
- **THEN** they point to the relevant `openspec/specs/` capability or active
  `openspec/changes/` artifact
- **AND** they do not point to `docs/spec/`, `plans/`, or `docs/superpowers/`
  as an authoritative contract source

#### Scenario: macOS shell contract references are checked
- **WHEN** macOS shell documentation or build/test metadata is updated
- **THEN** focused validation checks for stale active references to retired
  macOS shell contract paths
- **AND** any required compatibility bridge clearly states that OpenSpec wins
