---
name: __SKILL_NAME__
description: __SKILL_DESCRIPTION__
metadata:
  short-description: __SHORT_DESCRIPTION__
---

# __SKILL_NAME__

This package delegates execution to the package-local child agent `__SKILL_ID__`.

Use this skill when:
- __WHEN_TO_USE__

## Parent Runtime Contract

1. Keep the parent-side instructions short and stable.
2. Hand long-running or specialized work to the delegated child agent.
3. Return a bounded result to the parent runtime.
4. Move detailed material into `references/` and deterministic helpers into `scripts/`.
