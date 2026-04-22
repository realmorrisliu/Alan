---
name: skill-creator
description: |
  Design, scaffold, validate, and iterate on Alan skill packages.

  Use this when:
  - The user wants to create a new skill package
  - The user wants to update or refactor an existing skill package
  - You need to structure SKILL.md, sidecars, scripts, references, or assets
  - You need to validate package shape, execution metadata, or description fit
  - You need to plan explicit authoring and evaluation workflows for a skill

metadata:
  short-description: Create or update Alan skill packages
  tags: [skills, authoring, scaffolding, validation, eval]
capabilities:
  required_tools: [bash]
compatibility:
  requirements: Use the local `alan` CLI on PATH for init, validate, and eval helper flows.
---

# Skill Creator

This skill helps author ordinary directory-backed Alan skill packages.

## Working Model

Treat every skill as a single `skill package` with one root `SKILL.md`.

Package-local surfaces:

- `SKILL.md`: portable selection contract and core workflow
- `skill.yaml` / `package.yaml`: Alan-native runtime defaults
- `scripts/`: package-private deterministic helpers when a Rust CLI/bin is not
  the better fit
- `references/`: material to load only when needed
- `assets/`: templates or output resources
- `agents/`: child-agent roots and authoring metadata such as `openai.yaml`
- `evals/` and `eval-viewer/`: explicit authoring and review surfaces

Do not invent a second abstraction for authoring. Prefer tightening the package
shape and using explicit tooling.

## Authoring Workflow

1. Clarify the user task and what the description should communicate.
2. Pick a short package name in lowercase hyphen-case.
3. Scaffold the package with `alan skills init`.
4. Keep `SKILL.md` lean. Move detailed reference material into `references/`.
5. Prefer reusable Rust CLI/bin tooling first; keep only package-private or
   ecosystem-bound helpers in `scripts/`.
6. If the skill delegates, export a package-local child agent under `agents/`.
7. Validate with `alan skills validate --path <package>`.
8. Run explicit evaluation with `alan skills eval --path <package>`.

## Resource Guide

- Read `references/authoring.md` for package design guidance.
- Read `references/openai_yaml.md` before editing `agents/openai.yaml`.
- Read `references/schemas.md` for the structured eval manifest shape.
- Reuse `assets/templates/` when you need starter package content.
- Use `alan skills init`, `alan skills validate`, and `alan skills eval` for
  the primary authoring flow.
- Use the package-local compatibility wrappers under `scripts/` for review
  artifact regeneration.

## Rules

1. Prefer one package over sprawling nested abstractions.
2. Keep runtime tools, package-local helpers, and shared authoring tooling
   separate.
3. Prefer Rust CLI/bin surfaces over shell, Python, or TypeScript helpers
   unless an external ecosystem or a tiny package-private step makes a script
   the better fit.
4. If shared authoring or eval helpers move into Rust, prefer consolidating
   them into existing packages such as `alan-tools` or the `alan` CLI rather
   than introducing another standalone helper package.
5. Do not auto-load authoring or eval assets into the runtime prompt.
6. Make `description` concrete enough that catalog-based selection stays
   reliable.
