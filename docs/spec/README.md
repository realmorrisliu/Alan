# Legacy Spec Migration Bridge

`docs/spec/` is no longer the home for alan specifications.

Durable contracts belong in [OpenSpec long-lived specs](../../openspec/specs/).
In-flight design, task, verification, and requirement-delta work belongs in
[active OpenSpec changes](../../openspec/changes/).

This directory remains temporarily as stable bridge pages for older links.
Files here must not be treated as authoritative when they conflict with
OpenSpec.

## Current Migration Owner

- [documentation-governance](../../openspec/specs/documentation-governance/)

## Reader Guidance

- To understand shipped behavior, start from current implementation guides such
  as [Architecture](../architecture.md), [Current Governance Contract](../governance_current_contract.md),
  and [Skills And Tools](../skills_and_tools.md).
- To change target behavior, create or update an OpenSpec change.
- To preserve useful material from a legacy `docs/spec/*.md` file, move the
  normative requirements into the relevant OpenSpec capability and replace the
  old file with a short bridge only if active links still need it.
