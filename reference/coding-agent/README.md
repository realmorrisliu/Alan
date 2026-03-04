# Reference Coding Agent

This reference package demonstrates how to build a coding product layer on top of Alan runtime without forking kernel behavior.

## What this scaffold contains

1. A `profile.toml` with coding-oriented defaults (governance, tools, skills, extension slots).
2. A minimal coding skills pack (`decompose`, `edit-verify`, `deliver`).
3. Extension manifest examples for code index, test analysis, and PR helper capabilities.
4. Deterministic smoke and harness scripts under `scripts/reference` and `scripts/harness`.

## Quick run

1. `bash scripts/reference/run_coding_reference_smoke.sh`
2. `bash scripts/harness/run_coding_reference_suite.sh --ci-blocking`

## Directory map

- `profile.toml`: reference product profile.
- `skills/*/SKILL.md`: coding workflow skill pack.
- `extensions/*.yaml`: extension contract examples aligned with `docs/spec/extension_contract.md`.
