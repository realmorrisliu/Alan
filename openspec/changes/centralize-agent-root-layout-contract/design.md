## Context

The current canonical layout is `.alan/agents/default/` for the default agent and
`.alan/agents/<name>/` for named agents. The layout itself is now clearer, but the
implementation still lets many modules construct agent-root paths directly. Runtime
helpers exist, yet CLI, daemon, tests, and TUI code can still join literal path
segments such as `.alan`, `agents`, `default`, `skills`, `persona`, and `agent.toml`
on their own.

That creates an architectural amplification surface: a layout change requires many
call-site edits, and a missed writer can silently create files in the wrong location.
The desired state is that `alan-runtime` owns agent-root layout semantics, while outer
crates consume an explicit typed contract.

## Goals / Non-Goals

**Goals:**

- Make `alan-runtime` the single Rust source of truth for agent-root layout.
- Provide typed helpers for every canonical agent-root asset path: root,
  `agent.toml`, `persona/`, `skills/`, and `policy.yaml`.
- Make omitted `agent_name`, explicit `default`, and named agents flow through one
  normalization API.
- Convert CLI and daemon write/read sites to the typed layout API.
- Add tests or checks that make new production-code raw layout strings visible in
  review.
- Keep TUI setup behavior aligned without duplicating more path knowledge than needed.

**Non-Goals:**

- Changing the public on-disk layout again.
- Reintroducing compatibility reads from `.alan/agent/`.
- Moving all workspace path helpers out of their current crates.
- Eliminating user-facing documentation examples of canonical paths.
- Sharing Rust constants directly with TypeScript through code generation unless that
  proves necessary.

## Decisions

### Introduce a typed `AgentRootLayout` API

Add a small runtime-owned type such as `AgentRootLayout` or `AgentRootPaths` with
methods for default and named roots:

```text
global_default_root()
workspace_default_root(workspace_alan_dir)
global_named_root(name)
workspace_named_root(workspace_alan_dir, name)
agent_config_path(root)
persona_dir(root)
skills_dir(root)
policy_path(root)
```

The current free functions can either delegate to this type or be folded into it.
The important boundary is that consumers ask for semantic paths rather than joining
layout segments directly.

Alternative considered: keep the current helper set and only add more tests. That
would catch some regressions, but it does not make ownership obvious at call sites.

### Keep agent-name normalization central

The `default` reservation and single-component validation should live beside the
layout API. Callers should not decide locally whether `default` is a named overlay or
whether a candidate name is safe.

Alternative considered: leave validation in daemon route helpers. That keeps local
error messages simple, but it duplicates the most fragile semantic rule.

### Make CLI and daemon writers depend on runtime layout

`alan init`, connection pinning, workspace resolver creation, skill catalog, and
skill override writes should all call runtime layout helpers. This keeps read and
write paths symmetrical and prevents setup flows from creating paths runtime will not
load.

Alternative considered: introduce a separate `alan` crate helper. That would improve
local deduplication but still leaves runtime and host crates with two layout owners.

### Treat TypeScript as a small explicit contract

The TUI currently needs a canonical default config path before the daemon is
necessarily running. For now, keep a small TUI path helper and tests, but document it
as a mirror of the runtime contract. Where daemon APIs already return canonical paths,
the TUI should display returned values instead of recomputing them.

Alternative considered: generate a shared constants file. That may be useful later,
but it adds build-system complexity for a small client surface.

### Add a guardrail for raw layout strings

Add a focused test or script that scans Rust production files for raw canonical
agent-root strings such as `.alan/agents/default`, `agents/default`, or
`.join("agents").join("default")` outside approved modules and tests. The allowlist
should include docs, OpenSpec changes, tests, and the runtime layout module.

Alternative considered: rely on review discipline. The previous change showed that
path strings are easy to copy, so a cheap mechanical guardrail is justified.

## Risks / Trade-offs

- Helper churn across many call sites -> Keep API small and add compatibility wrappers
  for existing helper names during the refactor.
- Over-abstracting simple paths -> Only model stable semantic assets, not every
  transient runtime path.
- Guardrail false positives -> Start with warning-style or focused test allowlists and
  tune before making it too broad.
- TUI still mirrors one path -> Keep it isolated in `config-path.ts` and prefer daemon
  returned paths for all online flows.

## Migration Plan

1. Add runtime layout type and central name normalization while preserving current
   helper functions as delegates.
2. Move runtime internals to the new type.
3. Convert CLI and daemon consumers in focused slices: init/workspace creation,
   connection pinning, skill catalog, skill override, session creation.
4. Update tests to use semantic helpers where possible, leaving literal paths only
   where testing the actual external contract.
5. Add raw-string guardrail and document the allowlist.
6. Archive once OpenSpec tasks and focused verification pass.

Rollback is straightforward because this change should not alter persisted data: keep
the existing helper functions and revert call-site conversions if the new API proves
awkward.

## Open Questions

- Should the guardrail live as a Rust test, a shell script under `scripts/`, or a
  `just` target?
- Should TUI eventually receive canonical setup paths from a lightweight CLI command
  rather than mirroring path construction?
