# Alan macOS Shell Contract

> Status: VNext product contract for the macOS Alan shell host.

## Goal

Define the product-layer contract for a macOS terminal host that:

1. behaves like a real terminal app for human operators,
2. remains centered on terminal tabs that can boot directly into `alan-tui`,
3. exposes a typed shell model and control surface that Alan can query and operate.

This contract is about the macOS shell host. It does not redefine Alan runtime
internals and it does not replace the current TUI with a custom chat UI.

## Non-Goals

This contract does not require:

1. a GUI-first transcript surface replacing `alan-tui`,
2. iOS parity with the macOS host,
3. full process resurrection after host restart,
4. browser-first or IDE-first expansion before the terminal shell is stable,
5. unrestricted shell introspection beyond the explicit metadata boundary.

## Product Thesis

Alan is a real terminal app whose shell is readable and operable by both
humans and agents.

## Companion UI / UX Contract

The detailed user-facing layout, sidebar model, toolbar behavior, terminology,
and visual hierarchy for the macOS app are defined in
`docs/spec/alan_macos_shell_ui_ux.md`.

This document remains authoritative for:

1. shell object model,
2. control-plane behavior,
3. state ownership and binding boundaries.

The companion UI / UX contract is authoritative for:

1. what a user sees and names as a space, tab, pane, or inspector,
2. how the sidebar, toolbar, content area, and inspector are organized,
3. how attention and Alan-specific status appear in the native app.

The macOS product therefore has two first-class operators:

1. the human, who needs a fast, keyboard-first, terminal-native workspace,
2. the agent, who needs stable object identities, structured shell state, and a
   bounded mutation surface.

The center of the product must remain a terminal canvas. Opening the app should
be able to land directly in a terminal tab running `alan-tui`.

## Layer Responsibilities

### Shell Host

The macOS host owns:

1. windows, spaces, tabs, pane trees, and panes,
2. focus, layout, persistence, and attention state,
3. shell metadata capture and projection,
4. the local shell control surface.

### Alan Runtime

Alan runtime owns:

1. sessions, turns, runs, yields, checkpoints, and event history,
2. the daemon protocol and session lifecycle,
3. the TUI interaction loop.

### Builtin Shell Capability Package

The builtin shell capability package owns:

1. the shell object-model instructions exposed to Alan,
2. the shell command vocabulary and routing heuristics,
3. query-before-mutate behavior and operator safety rules.

The package must be implemented as a builtin package such as
`builtin:alan-shell-control`, not as a workspace-local prototype.

## Core Principles

1. **Real terminal app first**. The host must still make sense as a terminal app
   even when no Alan session is running.
2. **Shell state and Alan state are separate**. A pane may expose Alan-related
   metadata, but a pane is not an Alan session.
3. **No UI scraping as the primary interface**. Alan must consume typed shell
   state rather than infer the shell by reading pixels or accessibility labels.
4. **Object identity over visible position**. Automation and routing must target
   stable IDs rather than tab order alone.
5. **Bounded metadata exposure**. The shell may expose summaries and selected
   metadata, but must not imply unrestricted access to all terminal contents.

## Terminology

This contract uses `Tab` as the single term for the top-level work object
inside a space.

Older drafts may mention `Surface`. Treat that as historical wording rather
than an active compatibility name in this spec.

Rules:

1. `Tab` is the default term for product, UI, design, and repository-facing
   discussion.
2. Repo symbols, comments, docs, and APIs should use `tab` naming for this
   object.
3. CLI and control-plane operations should use `tab` naming consistently.

## Canonical Object Model

```text
AppWindow
  -> Space
     -> Tab
        -> PaneTree
           -> Pane
              -> ProcessBinding
              -> AlanBinding?
              -> AttentionState
              -> ViewportSnapshot
```

### AppWindow

Native macOS window that hosts one or more spaces.

### Space

Durable sidebar-level organizational unit.

Rules:

1. A space is the primary durable unit exposed in the sidebar.
2. A space may contain multiple tabs.
3. A space identity must remain stable across sidebar reordering and restarts
   when restored.

### Tab

Top-level tab-like object inside a space.

Kinds initially supported by this contract:

1. `terminal`
2. `scratch`
3. `log`

Additional kinds are additive. A browser tab kind is explicitly optional for the
first milestone.

### PaneTree

Split topology for a tab.

Rules:

1. A tab owns exactly one pane tree.
2. Internal tree nodes describe split orientation and child ordering.
3. Leaf nodes are panes and are the actual focus/action targets.

### Pane

Addressable leaf node in the pane tree.

Rules:

1. A pane is the smallest shell object that can receive focus or input.
2. A pane may host `alan-tui`, a shell, or another terminal process.
3. A pane identity is owned by the shell, not by Alan runtime.

### ProcessBinding

Observed process-level metadata for a pane.

Minimum fields:

1. `program`
2. `argv_preview?`
3. `cwd?`

### AlanBinding

Optional metadata projected onto a pane when the shell can reliably associate
that pane with Alan state.

This metadata is additive only. It must not replace pane identity.

### AttentionState

User-facing urgency and waiting state for shell routing.

Minimum states:

1. `idle`
2. `active`
3. `awaiting_user`
4. `notable`

### ViewportSnapshot

Safe shell summary describing what Alan can know about a pane without requiring
full scrollback access.

Suggested minimum fields:

1. `title?`
2. `summary?`
3. `visible_excerpt?`
4. `last_activity_at?`

## Identity and Relationship Rules

1. `Window -> Space -> Tab -> PaneTree -> Pane` is the shell model.
2. `Session -> Turn/Run -> Yield/Checkpoint -> Event history` is the Alan
   runtime model.
3. A pane may optionally carry `AlanBinding` metadata.
4. A pane is never the primary identifier for an Alan session.
5. An Alan session is never required for a pane to exist.
6. The shell must answer structural questions even when no Alan session is
   running.

## State Ownership

### Shell-Owned State

The shell host is authoritative for:

1. window identity and focus,
2. sidebar order and space membership,
3. tab identity and kind,
4. pane tree topology,
5. pane focus,
6. attention state,
7. shell-derived metadata,
8. restore/persistence metadata.

### Alan-Owned State

Alan runtime is authoritative for:

1. session identity,
2. run status,
3. yield status,
4. event history,
5. approval and structured-input state.

### Projected State

The shell may project bounded Alan metadata onto panes, such as:

1. `session_id`,
2. `run_status`,
3. `pending_yield`,
4. `latest_summary`.

Projected state is best treated as a cached view. It must not become the source
of truth for Alan runtime.

## Normative Shell Snapshot

The shell must expose a canonical state snapshot. The exact transport may vary,
but the model should be equivalent to the following shape:

```json
{
  "contract_version": "0.1",
  "window_id": "window_main",
  "focused_space_id": "space_alan_app",
  "focused_tab_id": "tab_main",
  "focused_pane_id": "pane_1",
  "spaces": [
    {
      "space_id": "space_alan_app",
      "title": "Alan App",
      "attention": "awaiting_user",
      "tabs": [
        {
          "tab_id": "tab_main",
          "kind": "terminal",
          "title": "Main Session",
          "pane_tree": {
            "node_id": "node_root",
            "kind": "split",
            "direction": "vertical",
            "children": [
              {"node_id": "pane_1", "kind": "pane"},
              {"node_id": "pane_2", "kind": "pane"}
            ]
          }
        }
      ]
    }
  ],
  "panes": [
    {
      "pane_id": "pane_1",
      "tab_id": "tab_main",
      "space_id": "space_alan_app",
      "cwd": "/Users/morris/Developer/Alan",
      "process": {"program": "alan-tui"},
      "attention": "awaiting_user",
      "viewport": {
        "title": "Alan",
        "summary": "waiting for approval",
        "last_activity_at": "2026-04-01T10:30:00Z"
      },
      "alan_binding": {
        "session_id": "sess_123",
        "run_status": "yielded",
        "pending_yield": true
      }
    },
    {
      "pane_id": "pane_2",
      "tab_id": "tab_main",
      "space_id": "space_alan_app",
      "cwd": "/Users/morris/Developer/Alan",
      "process": {"program": "zsh"},
      "attention": "idle",
      "viewport": {
        "title": "shell",
        "summary": "idle shell"
      }
    }
  ]
}
```

### Snapshot Field Contract

Required top-level fields:

1. `contract_version`
2. `window_id`
3. `focused_space_id?`
4. `focused_tab_id?`
5. `focused_pane_id?`
6. `spaces[]`
7. `panes[]`

Required `space` fields:

1. `space_id`
2. `title`
3. `attention`
4. `tabs[]`

Required `tab` fields:

1. `tab_id`
2. `kind`
3. `pane_tree`

Required `pane` fields:

1. `pane_id`
2. `tab_id`
3. `space_id`
4. `attention`
5. `process.program?`
6. `viewport.summary?`

Field rules:

1. IDs must be stable for the lifetime of the corresponding object.
2. Optional fields may be absent when not yet known.
3. Unknown metadata must not be represented as fabricated placeholder values.
4. `contract_version` should support additive evolution.
5. This contract version uses `tab` field names consistently.

## Minimum Questions The Shell Must Answer

The contract must support these questions without UI scraping:

1. How many spaces and tabs are open?
2. What split topology exists inside the active tab?
3. Which pane is focused?
4. What is happening in each pane at a summary level?
5. Which panes are waiting on user attention?
6. Which pane, if any, is currently associated with Alan state?

## Local Control Plane Contract

The shell must expose:

1. a local IPC surface,
2. a CLI wrapper for human and tool use.

This contract refers to the shipped CLI surface as `alan shell`. The shell may
still expose its own local IPC/socket layer internally, but human and agent
invocation must align with the `alan shell ...` namespace.

### IPC Envelope

The local IPC surface should support a simple request/response envelope.

Suggested request shape:

```json
{
  "contract_version": "0.1",
  "request_id": "req_123",
  "op": "pane.focus",
  "args": {
    "pane_id": "pane_1"
  }
}
```

Suggested success response:

```json
{
  "contract_version": "0.1",
  "request_id": "req_123",
  "ok": true,
  "result": {
    "focused_pane_id": "pane_1"
  }
}
```

Suggested error response:

```json
{
  "contract_version": "0.1",
  "request_id": "req_123",
  "ok": false,
  "error": {
    "code": "pane_not_found",
    "message": "pane_id pane_1 does not exist",
    "target_id": "pane_1"
  }
}
```

### Required Query Operations

1. `state`
2. `space list`
3. `tab list`
4. `pane list`
5. `pane snapshot --pane <id>`
6. `attention inbox`
7. `routing candidates`

Suggested operation names for IPC:

1. `shell.state`
2. `space.list`
3. `tab.list`
4. `pane.list`
5. `pane.snapshot`
6. `attention.inbox`
7. `routing.candidates`

### Required Mutation Operations

1. `space create`
2. `space open-alan`
3. `tab open`
4. `pane split`
5. `pane focus`
6. `pane send-text`
7. `tab close`
8. `attention set`

Suggested operation names for IPC:

1. `space.create`
2. `space.open_alan`
3. `tab.open`
4. `pane.split`
5. `pane.focus`
6. `pane.send_text`
7. `tab.close`
8. `attention.set`

### Required Behavior Rules

1. Query commands must be safe to call repeatedly.
2. Mutation commands must target stable object IDs.
3. A mutation against a missing target must fail explicitly.
4. The control plane must not rely on tab order alone for routing.

### Suggested CLI Shape

```text
alan shell state
alan shell space list
alan shell pane list
alan shell pane snapshot --pane <id>
alan shell pane focus --pane <id>
alan shell pane split --pane <id> --direction horizontal
alan shell pane send-text --pane <id> --text "..."
alan shell space open-alan --cwd <path>
alan shell attention inbox
```

### Query Result Shapes

`alan shell state` should return the canonical shell snapshot.

`alan shell pane list` should return a normalized pane collection:

```json
{
  "contract_version": "0.1",
  "panes": [
    {
      "pane_id": "pane_1",
      "space_id": "space_alan_app",
      "tab_id": "tab_main",
      "process": {"program": "alan-tui"},
      "attention": "awaiting_user",
      "alan_binding": {
        "session_id": "sess_123",
        "run_status": "yielded"
      }
    }
  ]
}
```

`alan shell pane snapshot --pane <id>` should return:

```json
{
  "contract_version": "0.1",
  "pane": {
    "pane_id": "pane_1",
    "cwd": "/Users/morris/Developer/Alan",
    "attention": "awaiting_user",
    "viewport": {
      "title": "Alan",
      "summary": "waiting for approval"
    }
  },
  "snapshot_state": {
    "stale": false
  }
}
```

`alan shell attention inbox` should return:

```json
{
  "contract_version": "0.1",
  "items": [
    {
      "item_id": "attn_1",
      "space_id": "space_alan_app",
      "tab_id": "tab_main",
      "pane_id": "pane_1",
      "attention": "awaiting_user",
      "summary": "approval requested"
    }
  ]
}
```

`alan shell routing candidates` should return:

```json
{
  "contract_version": "0.1",
  "target_kind": "pane",
  "candidates": [
    {
      "pane_id": "pane_1",
      "score": 1.0,
      "reasons": ["focused", "alan_binding:yielded", "attention:awaiting_user"]
    },
    {
      "pane_id": "pane_2",
      "score": 0.35,
      "reasons": ["same_tab"]
    }
  ]
}
```

### Mutation Result Shapes

`alan shell pane focus --pane <id>` should return:

```json
{
  "contract_version": "0.1",
  "applied": true,
  "focused_pane_id": "pane_1"
}
```

`alan shell pane split --pane <id> --direction horizontal` should return:

```json
{
  "contract_version": "0.1",
  "applied": true,
  "tab_id": "tab_main",
  "new_pane_id": "pane_3",
  "sibling_of": "pane_1"
}
```

`alan shell pane send-text --pane <id> --text "..."` should return:

```json
{
  "contract_version": "0.1",
  "applied": true,
  "pane_id": "pane_1",
  "accepted_bytes": 18
}
```

### Failure Semantics

Mutation and query failures should use machine-readable codes.

Suggested initial codes:

1. `pane_not_found`
2. `tab_not_found`
3. `space_not_found`
4. `invalid_direction`
5. `invalid_target_kind`
6. `stale_target`
7. `unsupported_tab_kind`
8. `busy`
9. `permission_denied`

## Event Subscription Contract

Polling is sufficient for the first slice, but the shell contract must reserve a
path for event subscription.

Required event families:

1. focus changed,
2. pane title changed,
3. cwd changed,
4. attention changed,
5. tab created,
6. tab closed,
7. Alan binding changed.

### Event Envelope

Suggested event shape:

```json
{
  "contract_version": "0.1",
  "event_id": "ev_123",
  "type": "attention.changed",
  "timestamp": "2026-04-01T10:30:00Z",
  "window_id": "window_main",
  "space_id": "space_alan_app",
  "tab_id": "tab_main",
  "pane_id": "pane_1",
  "payload": {
    "previous": "active",
    "current": "awaiting_user"
  }
}
```

### Required Event Payloads

`focus.changed`:

```json
{
  "previous_pane_id": "pane_2",
  "current_pane_id": "pane_1"
}
```

`pane.metadata_changed`:

```json
{
  "pane_id": "pane_1",
  "changed_fields": ["cwd", "viewport.summary"]
}
```

`attention.changed`:

```json
{
  "previous": "active",
  "current": "awaiting_user"
}
```

`tab.created`:

```json
{
  "tab_id": "tab_2",
  "space_id": "space_alan_app",
  "kind": "terminal"
}
```

`AlanBinding.changed`:

```json
{
  "pane_id": "pane_1",
  "session_id": "sess_123",
  "run_status": "yielded",
  "pending_yield": true
}
```

## Metadata Contract

The shell must combine multiple metadata sources rather than relying on one
mechanism.

### Primary Sources

1. Ghostty shell integration for cwd and shell-context signals,
2. app-owned shell state for space/tab/pane identity and focus,
3. Alan runtime projection for bounded run/yield metadata,
4. explicit shell notifications for attention and notable events.

### Metadata Rules

1. Metadata should be normalized into the shell snapshot rather than leaked as
   source-specific side channels.
2. Missing metadata must be representable as absent or stale, not silently
   invented.
3. The shell may expose a summary or excerpt, but the default contract should
   not imply unrestricted raw scrollback access.
4. Sidebar rows and pane snapshots must be producible without scraping the UI.

## Alan Binding Contract

When the shell launches `alan-tui`, it must be able to pass shell identity into
that process environment.

Minimum environment contract:

```text
ALAN_SHELL_SOCKET=/path/to/socket
ALAN_SHELL_WINDOW_ID=window_main
ALAN_SHELL_SPACE_ID=space_alan_app
ALAN_SHELL_TAB_ID=tab_main
ALAN_SHELL_PANE_ID=pane_1
```

### Binding Rules

1. These variables identify shell location, not Alan session identity.
2. `AlanBinding` must remain optional on panes.
3. A pane may stop hosting Alan without losing pane identity.
4. Alan-related status projected into the shell must remain bounded and
   additive.

## Persistence Boundary

The first milestone should persist shell structure, not promise full process
continuity.

Persist:

1. spaces,
2. surface membership,
3. pane tree topology,
4. working directory,
5. shell metadata needed for restore,
6. last known attention state,
7. Alan binding metadata when available.

Do not promise in this contract:

1. full process resurrection,
2. exact terminal buffer restoration for every process,
3. durable replay of arbitrary shell side effects.

## Safety and Privacy Boundary

1. The shell contract must make metadata exposure explicit.
2. Shell state may be queryable by Alan, but only through the bounded contract.
3. The host should prefer summaries, identity, and attention metadata over raw
   content whenever that is sufficient for routing.
4. High-risk mutations should remain explicit CLI/IPC operations, not hidden
   behind inferred UI gestures.

## Relationship To Existing Alan Contracts

This contract complements, but does not replace:

1. `docs/spec/app_server_protocol.md`
2. `docs/spec/reference_coding_agent.md`
3. `docs/spec/remote_control_architecture.md`
4. `docs/spec/capability_router.md`

The shell host is a product-layer host around the existing runtime and daemon,
not a runtime fork.

## Acceptance Criteria

1. The macOS shell is specified as a real terminal host rather than a transcript
   UI wrapper.
2. The shell object model defines `Window -> Space -> Tab -> PaneTree ->
   Pane` and keeps pane identity separate from Alan session identity.
3. The contract defines a canonical shell snapshot shape that can answer the
   product's core structural questions without UI scraping.
4. The control plane defines a minimum local IPC/CLI surface for query and
   mutation operations.
5. The metadata contract defines explicit source boundaries and stale-or-missing
   behavior.
6. The Alan binding contract defines how shell identity is injected into
   Alan-launched panes.
7. The persistence boundary is explicit about what is and is not restored in the
   first milestone.
8. The contract is sufficient to drive shell-host implementation from
   substrate through shell MVP and command-layer ergonomics.

## Future-Compatible Extensions

The following are explicitly additive and may arrive later without invalidating
this contract:

1. browser or inspector surfaces,
2. richer viewport summaries,
3. voice-first command-surface integration,
4. broader automation across non-terminal surfaces.
