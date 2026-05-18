## ADDED Requirements

### Requirement: Skill system contracts live in OpenSpec
alan SHALL specify skill package layout, `SKILL.md` semantics, compatibility
metadata, discovery, exposure, override behavior, prompt rendering, helper
assets, delegated launch targets, and management surfaces in OpenSpec.

#### Scenario: Skill package behavior changes
- **WHEN** a change modifies package discovery, frontmatter parsing,
  compatibility metadata, resource directories, built-in package distribution,
  skill availability, or skill execution
- **THEN** the OpenSpec delta updates this capability or another named skill
  capability
- **AND** `docs/skill_authoring.md` and `docs/skills_and_tools.md` remain
  implementation/operator guides instead of contract sources

#### Scenario: Legacy skill contract is opened
- **WHEN** `docs/spec/skill_system_contract.md` is reached during migration
- **THEN** the page points to this OpenSpec capability and does not restate the
  full legacy contract

### Requirement: Skill packages are directory-backed capabilities
alan SHALL treat a skill package as a directory with a root `SKILL.md` and
optional sidecars, resources, helper executables, evaluations, and package-local
agent launch targets.

#### Scenario: Portable skill is discovered
- **WHEN** alan discovers a directory containing `SKILL.md`
- **THEN** it can adapt that directory as a skill package without requiring an
  alan-specific manifest for the portable baseline

#### Scenario: alan-native assets are present
- **WHEN** a package includes alan-native sidecars such as `skill.yaml`,
  `package.yaml`, `agents/`, `bin/`, `scripts/`, `references/`, or `evals/`
- **THEN** alan exposes only the supported runtime and authoring surfaces
  defined by OpenSpec
- **AND** shipping a helper file inside a package does not make it a host-global
  runtime tool

### Requirement: Skill exposure is resolved before prompt rendering
alan SHALL resolve skill availability, overrides, built-in package sources, and
package-local launch targets before rendering the active prompt catalog.

#### Scenario: Skill override is applied
- **WHEN** `enabled` or `allow_implicit_invocation` is set through an
  `agent.toml` skill override
- **THEN** alan applies the resolved skill-level exposure state consistently in
  prompt assembly, `alan skills` inspection, and runtime availability checks
