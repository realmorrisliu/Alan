---
name: skill-creator
description: |
  Design, scaffold, validate, and iterate on Alan skill packages.

  Use this when:
  - The user wants to create a new skill package
  - The user wants to update or refactor an existing skill package
  - You need to structure SKILL.md, sidecars, scripts, references, or assets
  - You need to validate package shape, execution metadata, or trigger quality
  - You need to plan explicit authoring and evaluation workflows for a skill

metadata:
  short-description: Create or update Alan skill packages
  tags: [skills, authoring, scaffolding, validation, eval]
capabilities:
  required_tools: [bash]
  triggers:
    keywords:
      [
        skill,
        skill package,
        create skill,
        update skill,
        author skill,
        validate skill,
      ]
    patterns:
      ["create.*skill", "update.*skill", "author.*skill", "validate.*skill"]
compatibility:
  requirements: Use the local `alan` CLI on PATH for init, validate, and eval helper flows.
---

# Skill Creator

This skill helps author ordinary directory-backed Alan skill packages.

## Working Model

Treat every skill as a single `skill package` with one root `SKILL.md`.

Package-local surfaces:

- `SKILL.md`: portable trigger contract and core workflow
- `skill.yaml` / `package.yaml`: Alan-native runtime defaults
- `scripts/`: deterministic helpers
- `references/`: material to load only when needed
- `assets/`: templates or output resources
- `agents/`: child-agent roots and authoring metadata such as `openai.yaml`
- `evals/` and `eval-viewer/`: explicit authoring and review surfaces

Do not invent a second abstraction for authoring. Prefer tightening the package
shape and using explicit tooling.

## Authoring Workflow

1. Clarify the user intent the skill should trigger on.
2. Pick a short package name in lowercase hyphen-case.
3. Scaffold the package with `alan skills init`.
4. Keep `SKILL.md` lean. Move detailed reference material into `references/`.
5. Put repeated deterministic logic in `scripts/`.
6. If the skill delegates, export a package-local child agent under `agents/`.
7. Validate with `alan skills validate --path <package>`.
8. Run explicit evaluation with `alan skills eval --path <package>`.

## Resource Guide

- Read `references/authoring.md` for package design guidance.
- Read `references/openai_yaml.md` before editing `agents/openai.yaml`.
- Read `references/schemas.md` for the structured eval manifest shape.
- Reuse `assets/templates/` when you need starter package content.
- Use `scripts/init_skill.py` and `scripts/quick_validate.py` for deterministic
  helper flows.
- Use `scripts/run_eval.py`, `scripts/aggregate_benchmark.py`, and
  `scripts/generate_review.py` for explicit eval and review loops.

## Rules

1. Prefer one package over sprawling nested abstractions.
2. Keep runtime tools, package-local helpers, and shared authoring tooling
   separate.
3. Do not auto-load authoring or eval assets into the runtime prompt.
4. Make trigger descriptions concrete enough that explicit and deterministic
   activation stay reliable.
