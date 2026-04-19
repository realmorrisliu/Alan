---
name: repo-coding
description: |
  Delegate bounded repo-scoped coding work to a fresh repo worker.

  Use this when:
  - Alan has already selected the target repo or directory
  - The task needs inspect -> plan -> edit -> verify -> deliver inside that bound scope
  - The work should run in a focused child runtime instead of the home-root steward

metadata:
  short-description: Launch a repo-scoped coding worker
  tags: [coding, delegation, repo-worker, verification]
capabilities:
  required_tools: [bash]
---

# Repo Coding

This first-party package is the parent-facing entry for repo-scoped coding
work.

## Working Model

1. Treat the parent Alan runtime as the coding steward.
2. Use this package when the task should move into a fresh repo worker bound to
   one repo or directory.
3. Keep launch inputs explicit: delegated task, cwd, workspace boundary, and
   approval scope.
4. Expect bounded result integration rather than inheriting the full child
   transcript into the parent tape.

## Repo-Worker Expectations

The repo worker should:

1. inspect local code and restate constraints,
2. decompose the change into short verifiable steps,
3. apply minimal edits,
4. run targeted verification,
5. return a concise delivery summary with residual risk.

## Package Resources

- Read `references/package.md` for the package map and local validation entrypoints.
- Use the package-local child launch target `repo-worker`.
- Keep repo-local edits bounded; return control when the task expands beyond
  delegated scope.
