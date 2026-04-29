## Context

Alan currently models default agent definitions and named agent definitions with two
different directory shapes:

```text
~/.alan/agent/
~/.alan/agents/<name>/
<workspace>/.alan/agent/
<workspace>/.alan/agents/<name>/
```

This makes `.alan/agent/` a special case even though it has the same internal shape
as every named agent root (`agent.toml`, `persona/`, `skills/`, `policy.yaml`). The
new layout makes every agent root live under `.alan/agents/`:

```text
~/.alan/agents/default/
~/.alan/agents/<name>/
<workspace>/.alan/agents/default/
<workspace>/.alan/agents/<name>/
```

The user explicitly accepts the breaking change, so this design does not preserve a
compatibility fallback from `.alan/agent/`.

## Goals / Non-Goals

**Goals:**

- Make the default agent an explicit reserved agent root named `default`.
- Remove all runtime, CLI, daemon, docs, test, and ignore-pattern dependence on
  `.alan/agent/`.
- Preserve the existing overlay model except for the default-root path.
- Keep omitted `agent_name` ergonomic while making `agent_name = "default"` equivalent.
- Make write paths and displayed paths match the canonical layout.

**Non-Goals:**

- Automatic migration of local or workspace files from `.alan/agent/` to
  `.alan/agents/default/`.
- Compatibility reads, merged fallback overlays, or precedence rules involving
  `.alan/agent/`.
- Changes to public `.agents/skills/` package installs.
- Changes to package-local child-agent roots under a skill package's `agents/`
  directory.

## Decisions

### Use `.alan/agents/default/` as the only default root

Alan will derive the global default root from `~/.alan/agents/default/` and the
workspace default root from `<workspace>/.alan/agents/default/`. The old singular
path will not be read or written.

Alternative considered: support both paths during a compatibility window. That would
reduce migration pain, but it would keep two ways to define the same base agent and
make overlay debugging harder. Since the breaking change is accepted, direct removal
is simpler and easier to reason about.

### Treat `default` as reserved, not as a normal named overlay

An omitted `agent_name` and an explicit `agent_name = "default"` will select the same
default chain:

```text
~/.alan/agents/default
    -> <workspace>/.alan/agents/default
```

For a named agent such as `reviewer`, Alan will keep the existing default-then-named
layering, with updated paths:

```text
~/.alan/agents/default
    -> <workspace>/.alan/agents/default
    -> ~/.alan/agents/reviewer
    -> <workspace>/.alan/agents/reviewer
```

Alternative considered: let `default` behave like a normal named agent layered on top
of a hidden base. That would reintroduce a hidden default concept, so the reserved
name is cleaner.

### Keep internal root kinds semantic

The implementation can either rename `GlobalBase` / `WorkspaceBase` to
`GlobalDefault` / `WorkspaceDefault` or keep the old enum names temporarily while
changing paths. The preferred implementation is to rename labels and user-facing
strings to "default" because this change is intended to remove the confusing base vs
named vocabulary.

### Update all writers before readers are considered complete

Any command or API that creates default agent config, persona files, skills, policy
files, or skill overrides must write under `.alan/agents/default/`. A partial change
that only updates runtime reads would leave newly initialized installations in the old
layout, so write surfaces are part of the core scope.

### Detecting the old path is diagnostic only

Implementations may surface a warning when `.alan/agent/` exists, but they must not
load, merge, migrate, or otherwise use files from it. Diagnostics are allowed only to
help users understand why old definitions no longer apply.

## Risks / Trade-offs

- Existing installs stop loading config/persona/skills from `.alan/agent/` ->
  Mitigation: document the manual move to `.alan/agents/default/` and update setup
  errors/help text to name the new path.
- Hidden references in tests/docs keep creating old roots -> Mitigation: add repo-wide
  tests/search checks for `.alan/agent` references, allowing only migration notes or
  explicit negative tests.
- API clients may omit `agent_name` today -> Mitigation: keep omitted `agent_name`
  selecting the default agent while accepting explicit `"default"` as the same agent.
- Source-control rules may accidentally re-allow `.alan/agent/` -> Mitigation:
  update `.gitignore` to allow `.alan/agents/**` and `.alan/models.toml`, but not the
  singular root.

## Migration Plan

1. Update runtime path helpers and agent-root resolution to produce only
   `.alan/agents/default/` for default roots.
2. Update all write surfaces to create files in the new default root.
3. Update docs, examples, tests, and fixture paths.
4. Add negative tests proving `.alan/agent/` is ignored.
5. Users migrate manually with an equivalent move, for example:

   ```bash
   mkdir -p .alan/agents
   mv .alan/agent .alan/agents/default
   ```

Rollback means running an older Alan build and moving files back to `.alan/agent/`.

## Open Questions

- None for this proposal. The breaking-change policy is explicit: no compatibility
  fallback from `.alan/agent/`.
