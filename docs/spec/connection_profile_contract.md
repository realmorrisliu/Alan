# Connection Profile Contract

> Status: partially implemented V1 contract.
>
> Current reality: connection metadata persistence, provider descriptors,
> profile selection/default/pin semantics, daemon control-plane routes, CLI
> commands, and session profile binding are already implemented. Full migration
> away from legacy inline provider config and final onboarding/TUI cleanup
> remains in progress.
>
> Scope: unified operator-facing management of provider selection, provider
> configuration, credentials, login flows, onboarding, and session binding.

This document defines the operator-facing connection-management layer that sits
above the existing provider/auth contract in
[`provider_auth_contract.md`](./provider_auth_contract.md).

It does not collapse provider semantics. It standardizes how users create,
authenticate, select, inspect, and bind provider-backed connections across
CLI, TUI, onboarding, and daemon APIs.

## Current Implementation Snapshot

Implemented in the current tree:

1. `~/.alan/connections.toml` stores profile/credential metadata, while
   secret-bearing credentials and managed ChatGPT auth remain in separate host
   stores.
2. The daemon exposes the generic `/api/v1/connections/*` control plane for
   catalog, CRUD, current/default/pin selection, credential status, login,
   logout, secret entry, test, and event replay/streaming.
3. `alan connection ...` is the primary CLI surface for the same model.
4. Session creation already accepts `profile_id?` and persists
   `profile_id/provider/resolved_model` in session metadata.
5. `agent.toml.connection_profile` is already used as the pin field for global
   and workspace scopes.

Still in migration:

1. Runtime config still accepts legacy inline provider fields for backward
   compatibility, and applies resolved connection profiles onto that config.
2. Onboarding and all client UX surfaces are not yet reduced to the
   connection-profile-only flow described later in this document.
3. The "breaking change" section below remains target-state, not completed
   product behavior.

## Problem Statement

Historically Alan split the user-facing model-access story across three
unrelated surfaces:

1. `agent.toml` selects `llm_provider` and carries most provider config.
2. `/auth` and `alan auth` manage ChatGPT login only.
3. onboarding writes provider-specific config directly and does not define a
   reusable connection object.

This produces confusing states such as:

1. login succeeds but the active session still uses a different provider
2. onboarding and runtime do not share one source of truth
3. provider-specific auth UX grows by special-case branching instead of a
   stable host control plane

Current code has started closing those gaps through the connection control
plane, but migration is incomplete enough that the ambiguity above can still
surface in compatibility paths.

## Goals

Alan's connection-management contract must satisfy all of the following:

1. Present one uniform operator model for provider setup across CLI, TUI, and
   onboarding.
2. Keep provider semantics explicit. `chatgpt` remains distinct from
   `openai_*`, `anthropic_messages`, and `google_gemini_generate_content`.
3. Separate non-secret connection metadata from secret or managed credential
   state.
4. Make session binding explicit so `login`, `default set`, `pin`, and `new session`
   remain distinct actions.
5. Support multiple saved connections for the same provider family.
6. Establish one canonical operator-facing config format with no parallel
   legacy shape.

## Non-Goals

This contract does not require:

1. collapsing ChatGPT and API Platform auth into one provider
2. moving credentials into `agent.toml`
3. automatic live provider switching inside an already-running session
4. forcing one universal secret backend in V1 beyond the logical storage model

## Stable Vocabulary

- **Provider descriptor**: static metadata that describes a provider family,
  supported credential kinds, editable settings, login capabilities, and
  validation behavior.
- **Credential**: secret or managed-auth material used to authenticate provider
  requests. Examples: API key, managed OAuth login state, or ambient cloud
  auth.
- **Credential status**: host-managed availability state for a credential such
  as `missing`, `available`, `pending`, `expired`, or `error`.
- **Connection profile**: a named, user-selectable provider configuration that
  references a credential and carries non-secret settings such as base URL,
  model, region, workspace binding, or client name.
- **Default profile**: the profile chosen by the operator as the default for
  new sessions when no explicit profile is supplied.
- **Pinned profile**: an optional `connection_profile` stored in an
  `agent.toml` file that overrides the default profile for the corresponding
  global or workspace agent scope.
- **Effective profile**: the profile Alan will use for the next session after
  resolving explicit session input, pin state, and default-profile state.
- **Session binding**: the frozen association between a session and the
  connection profile used to create it.

## Core Invariants

The following rules are normative:

1. `login` only changes credential state.
2. `default set` only changes the default profile for future session creation.
3. `pin` only changes `agent.toml.connection_profile` for the chosen scope.
4. Existing sessions must not switch providers implicitly when credential or
   default-profile state changes.
5. Session creation must resolve one concrete profile and freeze a provider
   snapshot into session metadata.
6. Credentials must remain outside `agent.toml`.
7. `agent.toml` may optionally pin a connection profile, but it must not carry
   inline provider-specific connection fields.
8. The connection layer may unify UX, but it must not erase provider-family
   differences defined in [`provider_auth_contract.md`](./provider_auth_contract.md).

## Logical Model

### Provider Descriptor

The host exposes a catalog of provider descriptors. A descriptor is static
metadata, not operator state.

Stable fields:

```text
provider_id
display_name
credential_kind
supports_browser_login
supports_device_login
supports_secret_entry
supports_logout
supports_test
required_settings[]
optional_settings[]
default_settings{}
```

V1 provider catalog:

| Provider | Credential Kind | Interactive Login | Secret Entry | Notes |
| --- | --- | --- | --- | --- |
| `chatgpt` | `managed_oauth` | browser, device | no | uses managed ChatGPT/Codex login |
| `openai_responses` | `secret_string` | no | yes | API key |
| `openai_chat_completions` | `secret_string` | no | yes | API key |
| `openai_chat_completions_compatible` | `secret_string` | no | yes | API key or token-like secret |
| `openrouter` | `secret_string` | no | yes | OpenRouter API key; default model `moonshotai/kimi-k2.6` |
| `anthropic_messages` | `secret_string` | no | yes | API key or provider-compatible secret |
| `google_gemini_generate_content` | `ambient_cloud_auth` | no | no | requires project/location/model and valid local Google auth |

Credential kinds are logical classes:

1. `managed_oauth`
2. `secret_string`
3. `ambient_cloud_auth`

Future credential kinds may be added without changing the profile contract.

### Credential

A credential is identified by `credential_id`.

Stable logical fields:

```text
credential_id
credential_kind
provider_family
label
backend
status
```

Rules:

1. Multiple profiles may reference the same credential.
2. A credential may outlive any one profile.
3. Credential status is host-owned runtime state, not agent-definition state.

### Connection Profile

A connection profile is the primary operator-facing object.

Stable fields:

```text
profile_id
label?
provider
credential_id
settings{}
created_at
updated_at
source
```

Rules:

1. `profile_id` is stable and operator-visible.
2. `label` is optional display metadata and must not be used as a stable key.
3. `provider` chooses the request/auth surface.
4. `settings` contains only non-secret provider configuration.
5. `credential_id` is required unless the provider descriptor explicitly allows
   `ambient_cloud_auth` without an explicit secret-bearing credential.
6. Profile settings must be validated against the provider descriptor.

### Session Binding

When a session starts, Alan resolves a single profile and freezes a binding:

```text
session_id
profile_id
provider
settings_snapshot{}
credential_snapshot_kind
resolved_model
```

Rules:

1. Session binding is immutable for the lifetime of that session.
2. Mutating a profile affects future sessions only.
3. UI surfaces must show the bound profile and provider for the current
   session.

### Selection Resolution

For new-session creation, Alan resolves profiles in this precedence order:

1. explicit session-create `profile_id`
2. workspace pin from `{workspace}/.alan/agents/default/agent.toml`
3. global pin from `~/.alan/agents/default/agent.toml`
4. `~/.alan/connections.toml.default_profile`

Rules:

1. `default_profile` is the operator default, not a pin.
2. `pin` is an explicit override and must win over `default_profile`.
3. onboarding must set `default_profile` but must not write a pin unless the
   operator explicitly asks to pin.
4. `current` inspection surfaces must report global pin, workspace pin,
   default profile, and effective profile separately.

## Persistence Contract

### Non-Secret Metadata

V1 stores connection metadata in:

```text
~/.alan/connections.toml
```

Canonical V1 shape:

```toml
version = 1
default_profile = "chatgpt-main"

[credentials.chatgpt]
kind = "managed_oauth"
provider_family = "chatgpt"
label = "ChatGPT login"
backend = "alan_home_auth_json"

[credentials.kimi-key]
kind = "secret_string"
provider_family = "anthropic_messages"
label = "Kimi Coding API key"
backend = "alan_home_secret_store"

[credentials.openrouter-key]
kind = "secret_string"
provider_family = "openrouter"
label = "OpenRouter API key"
backend = "alan_home_secret_store"

[profiles.chatgpt-main]
provider = "chatgpt"
credential_id = "chatgpt"
source = "managed"

[profiles.chatgpt-main.settings]
base_url = "https://chatgpt.com/backend-api/codex"
model = "gpt-5.3-codex"
account_id = ""

[profiles.kimi]
provider = "anthropic_messages"
credential_id = "kimi-key"
source = "managed"

[profiles.kimi.settings]
base_url = "https://api.kimi.com/coding"
model = "k2p5"
client_name = ""
user_agent = ""

[profiles.openrouter-main]
provider = "openrouter"
credential_id = "openrouter-key"
source = "managed"

[profiles.openrouter-main.settings]
base_url = "https://openrouter.ai/api/v1"
model = "moonshotai/kimi-k2.6"
http_referer = ""
x_title = ""
app_categories = ""
```

### Secret And Managed Credential Storage

V1 logical storage contract:

1. Non-secret metadata lives in `connections.toml`.
2. Secret-bearing credentials live in a host-managed store outside
   `agent.toml`.
3. Managed ChatGPT login state remains outside `connections.toml`.

V1 concrete backend choices:

1. Existing ChatGPT managed login continues to use `~/.alan/auth.json`.
2. New `secret_string` credentials use a host-managed secret store under Alan
   home with file permissions equivalent to `0600`.
3. Future host implementations may replace the secret backend with keychain or
   keyring integration without changing the logical contract.

## Agent Config Contract

`agent.toml` remains the agent-definition file. The current implementation is
moving it toward a provider-agnostic shape, but legacy inline provider fields
still exist during migration.

Target V1 shape:

```toml
llm_request_timeout_secs = 180
tool_timeout_secs = 30
max_tool_loops = 0
tool_repeat_limit = 4
context_window_tokens = 128000
prompt_snapshot_enabled = false
prompt_snapshot_max_chars = 8000

[memory]
enabled = true
strict_workspace = true
```

Rules:

1. `connection_profile` is optional and means "pin this agent/workspace to a
   specific profile".
2. Target end-state: provider-specific settings such as `base_url`, `model`,
   `account_id`, `project_id`, or `api_key` disappear from the user-facing
   `agent.toml` surface. Current code still accepts them for backward
   compatibility.
3. Agent overlays may change `connection_profile`, but they should do so
   explicitly as a profile reference rather than by reintroducing inline
   provider keys.
4. Runtime-only knobs such as timeouts, compaction thresholds, memory, and
   skill overrides remain in `agent.toml`.

## Resolved Configuration Rules

Alan must resolve the active profile for a new session in this order:

1. explicit session `profile_id`
2. workspace agent config `connection_profile`
3. global agent config `connection_profile`
4. `connections.toml` `default_profile`
5. onboarding-required / configuration-required failure

Rules:

1. `connection_profile` is the canonical config field in resolved agent config.
2. If neither `connection_profile` nor `default_profile` is available, session
   creation must fail with a configuration-required error.
3. The runtime must not synthesize fallback profiles from removed inline
   provider keys.

## Host Control Plane

The daemon exposes a generic connection-management surface.

Canonical V1 routes:

1. `GET /api/v1/connections/catalog`
2. `GET /api/v1/connections`
3. `GET /api/v1/connections/current`
4. `POST /api/v1/connections/default/set`
5. `POST /api/v1/connections/default/clear`
6. `POST /api/v1/connections/pin`
7. `POST /api/v1/connections/unpin`
8. `POST /api/v1/connections`
9. `GET /api/v1/connections/{profile_id}`
10. `PATCH /api/v1/connections/{profile_id}`
11. `DELETE /api/v1/connections/{profile_id}`
12. `POST /api/v1/connections/{profile_id}/activate`
13. `GET /api/v1/connections/{profile_id}/credential/status`
14. `POST /api/v1/connections/{profile_id}/credential/login/browser/start`
15. `POST /api/v1/connections/{profile_id}/credential/login/device/start`
16. `POST /api/v1/connections/{profile_id}/credential/login/device/complete`
17. `POST /api/v1/connections/{profile_id}/credential/logout`
18. `POST /api/v1/connections/{profile_id}/credential/secret`
19. `POST /api/v1/connections/{profile_id}/test`
20. `GET /api/v1/connections/events`
21. `GET /api/v1/connections/events/read`

### Canonical Host Objects

Provider descriptor response shape:

```json
{
  "provider_id": "chatgpt",
  "display_name": "ChatGPT / Codex",
  "credential_kind": "managed_oauth",
  "supports_browser_login": true,
  "supports_device_login": true,
  "supports_secret_entry": false,
  "supports_logout": true,
  "supports_test": true,
  "required_settings": ["base_url", "model"],
  "optional_settings": ["account_id"],
  "default_settings": {
    "base_url": "https://chatgpt.com/backend-api/codex",
    "model": "gpt-5.3-codex",
    "account_id": ""
  }
}
```

ChatGPT/Codex model identifiers must track upstream Codex bundled model
metadata. Alan must not invent synthetic ChatGPT model slugs. At the time of
this spec revision, upstream bundled examples include `gpt-5.3-codex`,
`gpt-5.2-codex`, `gpt-5.1-codex-max`, and `gpt-5.1-codex-mini`.

The OpenRouter descriptor uses `provider_id = "openrouter"`, secret-string
credentials, required `model`, optional `base_url`, `http_referer`, `x_title`,
and `app_categories`, and defaults to `model = "moonshotai/kimi-k2.6"` with
`base_url = "https://openrouter.ai/api/v1"`.

Connection profile summary shape:

```json
{
  "profile_id": "chatgpt-main",
  "provider": "chatgpt",
  "credential_id": "chatgpt",
  "settings": {
    "base_url": "https://chatgpt.com/backend-api/codex",
    "model": "gpt-5.3-codex",
    "account_id": ""
  },
  "credential_status": "available",
  "is_default": true,
  "source": "managed",
  "created_at": "2026-04-10T06:00:00Z",
  "updated_at": "2026-04-10T06:00:00Z"
}
```

Connection-selection shape:

```json
{
  "workspace_dir": "/Users/morris/Developer/Alan",
  "global_pin": {
    "scope": "global",
    "config_path": "/Users/morris/.alan/agents/default/agent.toml",
    "profile_id": "kimi"
  },
  "workspace_pin": null,
  "default_profile": "chatgpt",
  "effective_profile": "kimi",
  "effective_source": "global_pin"
}
```

Credential-status shape:

```json
{
  "profile_id": "chatgpt-main",
  "credential_id": "chatgpt",
  "credential_kind": "managed_oauth",
  "status": "available",
  "last_checked_at": "2026-04-10T06:05:00Z",
  "detail": {
    "account_email": "morrisliu1994@outlook.com",
    "account_plan": "pro"
  }
}
```

Connection-test result shape:

```json
{
  "profile_id": "chatgpt-main",
  "ok": true,
  "provider": "chatgpt",
  "resolved_model": "gpt-5.3-codex",
  "message": "Connection test succeeded."
}
```

Create/update payload rules:

1. `POST /api/v1/connections` accepts `profile_id`, optional `label`, `provider`,
   optional `credential_id`, and `settings`.
2. `PATCH /api/v1/connections/{profile_id}` may mutate `label`, `settings`,
   and `credential_id`, but it must not mutate `provider`.
3. Changing `provider` requires creating a new profile so session history and
   operator intent remain explicit.

### Required Semantics

1. `catalog` returns provider descriptors only.
2. `connections` returns profile summaries plus default-profile metadata.
3. `default set` changes the default profile but never mutates existing
   sessions.
4. login routes are available only when the provider descriptor supports the
   selected credential kind.
5. `credential/secret` is available only for `secret_string` credentials.
6. `test` performs a provider-specific dry run or minimal validation and must
   return errors in a provider-neutral host envelope.
7. `pin` and `unpin` modify `agent.toml.connection_profile` only; they must
   not mutate `default_profile`.

### Connection Events

The event stream must be generic, not ChatGPT-only.

Stable event types:

1. `profile_created`
2. `profile_updated`
3. `profile_deleted`
4. `profile_activated`
5. `credential_status_changed`
6. `login_started`
7. `browser_login_ready`
8. `device_code_ready`
9. `login_succeeded`
10. `login_failed`
11. `logout_completed`
12. `connection_test_succeeded`
13. `connection_test_failed`

Each event envelope must include:

1. `profile_id`
2. `provider`
3. `credential_id` when applicable
4. replay cursor metadata

### Error Model

The host control plane must return provider-neutral error envelopes with
stable error codes.

Minimum V1 error codes:

1. `profile_not_found`
2. `credential_not_found`
3. `provider_not_supported`
4. `unsupported_operation`
5. `validation_failed`
6. `credential_missing`
7. `credential_pending`
8. `credential_expired`
9. `login_failed`
10. `connection_test_failed`
11. `session_binding_conflict`

Canonical error shape:

```json
{
  "error": {
    "code": "credential_missing",
    "message": "Profile kimi does not have an available credential.",
    "profile_id": "kimi",
    "provider": "anthropic_messages",
    "retryable": false
  }
}
```

## CLI Contract

Canonical CLI namespace:

```text
alan connection list
alan connection show <profile-id>
alan connection current [--workspace <path>]
alan connection add <provider> [--profile <profile-id>]
alan connection edit <profile-id>
alan connection set-secret <profile-id>
alan connection login <profile-id> [browser|device]
alan connection logout <profile-id>
alan connection default set <profile-id>
alan connection default clear [--workspace <path>]
alan connection pin <profile-id> [--scope global|workspace] [--workspace <path>]
alan connection unpin [--scope global|workspace] [--workspace <path>]
alan connection test [<profile-id>]
alan connection remove <profile-id>
```

OpenRouter setup example:

```bash
alan connection add openrouter --profile openrouter-main --setting model=moonshotai/kimi-k2.6
alan connection set-secret openrouter-main
alan connection test openrouter-main
```

Rules:

1. `add` creates metadata only.
2. `set-secret` or `login` is a separate explicit step.
3. `default set` changes `default_profile` only.
4. `pin` and `unpin` are the only commands that modify
   `agent.toml.connection_profile`.
5. `current` prints global pin, workspace pin, default profile, and effective
   profile separately.
6. `activate` remains only as a compatibility alias for `default set`.
7. `alan auth` is removed from the primary product surface in favor of
   `alan connection`.

## TUI Contract

The TUI must use the same host control plane as the CLI and onboarding.

Canonical slash commands:

```text
/connection list
/connection show <profile-id>
/connection current
/connection add <provider>
/connection login <profile-id> [browser|device]
/connection set-secret <profile-id>
/connection default set <profile-id>
/connection default clear
/connection pin <profile-id> [scope=global|workspace]
/connection unpin [scope=global|workspace]
/connection status [<profile-id>]
/connection test [<profile-id>]
/connection remove <profile-id>
```

Rules:

1. The current session header must display the bound `profile_id`, provider,
   and resolved model.
2. After login succeeds, if the current session is bound to a different
   profile, the TUI must say so explicitly.
3. The success message must recommend `default set` plus `new session` when
   required.
4. The TUI must no longer imply that login alone changes the active model
   provider.
5. `status` remains only as a compatibility alias:
   `status <profile-id>` = `show <profile-id>`,
   `status` = `current`.
6. `use` remains only as a compatibility alias for `default set`.

Required post-login warning shape:

```text
ChatGPT login complete for profile chatgpt-main.
Current session is still using profile kimi (anthropic_messages).
Run /connection default set chatgpt-main and create a new session to use it.
```

`/auth` is removed from the primary TUI surface. Connection management is
owned by `/connection`.

## Onboarding Contract

Onboarding must become a connection-profile flow, not a one-shot config-file
writer.

Required behavior:

1. onboarding reads the provider catalog from the same descriptor model used by
   runtime UI
2. onboarding creates or selects one connection profile
3. onboarding optionally authenticates or stores the required secret
4. onboarding optionally marks the created profile as default
5. onboarding explains whether the first session will use that profile
6. onboarding must not write `agent.toml.connection_profile` unless the
   operator explicitly chooses to pin the agent/workspace

First-run paths:

1. no profiles:
   create a fresh profile
2. profiles already exist:
   allow selection, inspection, or creation of an additional profile

## Runtime Binding Contract

Session APIs must grow explicit profile support.

Required session-create input:

```text
profile_id? string
```

Required session metadata output:

```text
profile_id
provider
resolved_model
```

Rules:

1. If session creation omits `profile_id`, the host resolves one using the
   precedence rules above.
2. Rebinding requires starting a new session or forking into a new session with
   a different profile.

Canonical session metadata example:

```json
{
  "session_id": "31cefc7b-deb1-40cc-8167-dc7cb5f120f1",
  "profile_id": "chatgpt-main",
  "provider": "chatgpt",
  "resolved_model": "gpt-5.3-codex"
}
```

## Breaking Change Contract

This section describes the remaining migration target, not the current
compatibility surface.

V1 fully adopts the connection-profile format as the only supported
operator-facing shape.

Required replacements:

1. `llm_provider` is removed from user-facing `agent.toml`.
2. Provider-specific inline config keys are removed from user-facing
   `agent.toml`.
3. `connection_profile` becomes the only agent-facing model-selection field.
4. `/api/v1/auth/providers/chatgpt/...` is replaced by
   `/api/v1/connections/...`.
5. `alan auth` and `/auth` are replaced by `alan connection` and
   `/connection`.

File evolution:

1. `~/.alan/connections.toml` becomes the canonical provider/profile metadata
   file.
2. `~/.alan/auth.json` may remain as the concrete backend for managed ChatGPT
   credentials, but it is no longer a user-facing primary config surface.
3. Existing checked-in examples, tests, templates, and onboarding output should
   be rewritten to the connection-profile format in the same change series.

## Acceptance Criteria

This contract is satisfied when all of the following are true:

1. onboarding, CLI, and TUI all use one shared connection catalog model
2. at least two saved profiles for the same provider family can coexist
3. ChatGPT login success no longer implies provider activation
4. activating a profile does not mutate an existing session
5. new session creation can bind an explicit `profile_id`
6. current session UI always tells the operator which profile and provider are
   active
7. user-facing `agent.toml` no longer accepts inline provider-specific
   connection keys
8. the host control plane is no longer ChatGPT-only

## Relationship To Other Specs

1. [`provider_auth_contract.md`](./provider_auth_contract.md) remains the
   source of truth for provider/auth boundaries.
2. This document defines the operator-facing connection-management layer above
   that boundary.
3. [`app_server_protocol.md`](./app_server_protocol.md) should absorb the
   concrete wire shapes once the connection control plane is implemented.
