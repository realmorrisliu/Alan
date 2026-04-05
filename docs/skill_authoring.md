# Skill Authoring Workflow

Alan's first-party skill authoring tooling operates over the same
directory-backed `skill package` contract used by externally discovered skills.
It does not introduce a second package system.

## Package Model

Every skill package is one directory with one root `SKILL.md` plus optional
bundled assets:

```text
skill-name/
├── SKILL.md
├── skill.yaml
├── package.yaml
├── scripts/
├── references/
├── assets/
├── evals/
├── eval-viewer/
└── agents/
```

Use the root `SKILL.md` for the portable trigger contract. Keep it lean. Move
detailed reference material into `references/` and deterministic helpers into
`scripts/`.

Alan separates three things:

- host/runtime tools
- package-local helpers in `scripts/`
- reusable authoring/eval tooling shared across packages

Shipping a script inside a skill package does not create a new runtime tool.

## Commands

Create a new package from a first-party template:

```bash
alan skills init path/to/my-skill --template inline
alan skills init path/to/repo-review --template delegate
```

Validate package shape, metadata, compatibility sidecars, resources, and
execution shape:

```bash
alan skills validate path/to/my-skill
alan skills validate path/to/my-skill --json
alan skills validate path/to/my-skill --strict
```

Run explicit package-local evaluation hooks:

```bash
alan skills eval path/to/my-skill
alan skills eval path/to/my-skill --output-dir /tmp/skill-eval
alan skills eval path/to/my-skill --require-hook
```

## Templates

V1 ships two first-party templates:

- `inline`: one portable skill with no delegated child agent
- `delegate`: one portable skill plus one package-local child agent whose export
  name matches the package id

Both templates generate ordinary skill packages. They can be installed,
discovered, and mounted the same way as any other package.

## Validation

`alan skills validate` currently checks:

- root package shape and presence of `SKILL.md`
- `SKILL.md` frontmatter parsing and stable field validation
- `skill.yaml`, `package.yaml`, and `agents/openai.yaml` parseability
- resource-directory shape for `scripts/`, `references/`, `assets/`, `evals/`,
  and `eval-viewer/`
- child-agent export discovery under `agents/`
- resolved execution shape, including unresolved delegated-package diagnostics

Warnings remain non-fatal by default. `--strict` upgrades warnings to a failing
exit code.

## Eval Workbench

Evaluation remains explicit and package-local. Alan does not automatically load
grader/analyzer/comparator/review assets into runtime prompt context.

`alan skills eval` first runs validation, then prefers a structured manifest:

- `evals/evals.json`

Structured manifests currently support:

- `trigger` cases for deterministic trigger/description evaluation
- `command` cases for explicit candidate runs with optional baseline, grading,
  analyzer, and comparator stages
- comparison modes `with_without_skill`, `new_old_skill`, and `custom`

Structured eval writes stable artifacts under the selected output directory:

- `run.json`
- `benchmark.json`
- `review/index.html`
- per-case artifacts under `cases/<case-id>/`

If no manifest is present, Alan falls back to legacy hooks:

- `scripts/eval.sh`
- `scripts/eval.py`

Legacy hooks run with the package root as the current working directory and
export:

- `ALAN_SKILL_ID`
- `ALAN_SKILL_PACKAGE_ID`
- `ALAN_SKILL_PACKAGE_ROOT`

When no manifest or hook exists, `alan skills eval` reports `no_hook`. This is
non-fatal by default and becomes fatal with `--require-hook`.

## Shared Tooling

Alan keeps reusable authoring/eval helpers in `crates/skill-tools/`.

- `alan skills eval` uses the shared tooling library for manifest execution and
  artifact generation
- the `alan-skill-tools` binary exposes reusable helper commands such as
  `aggregate-benchmark` and `generate-review`
- package-local scripts can call those helpers without turning them into
  runtime tools or `alan` top-level commands

Promotion path:

- keep a helper in `scripts/` when it is private to one skill package
- move it into shared tooling when multiple skill packages need the same stable
  operator-side helper
- promote it into a runtime tool only when models need a uniform host-level
  capability rather than an explicit authoring workflow

## Authoring Guidance

- Treat `description` as the trigger contract. It should say what the skill does
  and when to use it.
- Keep `SKILL.md` short and procedural.
- Prefer deterministic scripts over repeatedly rewritten code.
- Use `references/` for detailed schemas, examples, and background material.
- Keep package clutter low. Do not add extra README/changelog/process-history
  files inside skill packages unless they are part of the skill itself.
