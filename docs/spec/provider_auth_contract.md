# Provider And Auth Contract

> Status: VNext contract for provider/auth layering.
> Scope: model-provider selection, authentication surfaces, and request-auth bridging outside the kernel.

## Goal

Keep Alan's model access surface small and explicit:

1. Provider selection chooses a concrete transport and auth surface.
2. API Platform auth and ChatGPT/Codex subscription auth remain separate.
3. Request projection and auth bridging may be provider-specific without contaminating Tape or kernel invariants.
4. Product-layer agents can rely on a stable provider/auth contract without forking `alan-runtime`.

## Layer Boundary

### Kernel MUST NOT own

1. Browser login, device-code login, refresh-token persistence, or account selection UX.
2. Provider-family billing semantics or subscription policy differences.
3. Product-specific decisions about which auth surface should be preferred.

### Runtime/provider layer MUST own

1. Provider-specific request projection.
2. Request authentication header construction.
3. Provider-specific auth refresh and auth-failure classification.
4. Mapping provider/auth state into transport requests.

### Host / CLI layer MUST own

1. Login and logout commands.
2. Local auth-state inspection.
3. Secure persistence of managed auth state outside `agent.toml`.

## Provider Surface

`llm_provider` is the top-level provider/auth selector.

Current target split:

1. `openai_responses`
   API Platform only.
2. `openai_chat_completions`
   API Platform only.
3. `openai_chat_completions_compatible`
   Generic compatible endpoints only.
4. `chatgpt`
   ChatGPT/Codex subscription auth surface, implemented separately from API-key providers.

Normative boundary:

1. `openai_*` providers must not depend on ChatGPT login state.
2. `chatgpt` must not read API keys from the OpenAI Platform config fields.
3. `openai_chat_completions_compatible` remains a generic endpoint family and must not imply ChatGPT semantics.

## Auth Surface Split

Alan recognizes two OpenAI-family auth classes:

1. **API Platform auth**
   Based on explicit API-key config in `agent.toml` or equivalent resolved config.
2. **ChatGPT/Codex subscription auth**
   Based on managed login state, bearer refresh, and ChatGPT account/workspace identity.

This split exists because ChatGPT and API Platform are separate operator surfaces with different billing, policy, and account semantics.

## ChatGPT Provider Contract

The `chatgpt` provider is a first-class provider surface with these invariants:

1. It may reuse the Responses wire shape where compatible.
2. It must still be represented as a distinct provider/auth surface from `openai_responses`.
3. It must support managed local login state rather than API-key-only config.
4. It must treat account/workspace identity as part of request auth context, not as prompt content.

Default transport assumptions for the experimental path:

1. Default base URL is ChatGPT/Codex-specific rather than `https://api.openai.com/v1`.
2. Requests use bearer auth derived from managed ChatGPT login state.
3. Requests may include ChatGPT account/workspace identity headers when required by the provider surface.

## Auth State Storage Contract

Managed ChatGPT auth state must live outside resolved agent config:

1. It must not be stored in `agent.toml`.
2. It must not be treated as part of agent identity.
3. It should be stored under Alan home, for example `~/.alan/auth.json`, keyring, or an equivalent managed store.

Rationale:

1. Provider login state is operator-local state, not agent-definition state.
2. This keeps checked-in/workspace agent roots free of local secrets.
3. It matches the existing Alan split between agent-facing config and machine-local host state.

## Login Flows

The local managed ChatGPT path should support:

1. Browser-based login as the primary interactive flow.
2. Device-code login as the headless fallback.
3. Explicit logout.
4. Explicit status inspection.

## Host Auth Control Plane

When Alan is hosted behind a daemon or app-server, the host layer may expose the managed
ChatGPT auth state through an explicit control plane.

Normative behavior:

1. The control plane must sit on top of the same managed auth core used by the local CLI flow.
2. Host routes must not introduce a second token store or a second refresh implementation.
3. The minimum host surface is:
   `status`, `logout`, `login start`, auth completion handling, and auth event observation.
4. Host auth observation and mutation must be independently scope-gated from session I/O.
5. Alan's current host scope names are `host.auth.read` and `host.auth.write`.
6. Browser login should support a daemon-owned callback path so UI clients only need to start the
   flow, open the returned `auth_url`, and observe completion through host events/status.
7. A daemon-owned browser callback endpoint may be exempt from bearer-token scope checks, but it
   must be bound to a pending login attempt and validated with OAuth state before mutating auth
   state.

Current daemon surface shape:

1. `GET /api/v1/auth/providers/chatgpt/status`
2. `POST /api/v1/auth/providers/chatgpt/logout`
3. `GET /api/v1/auth/providers/chatgpt/events`
4. `GET /api/v1/auth/providers/chatgpt/events/read`
5. `POST /api/v1/auth/providers/chatgpt/login/device/start`
6. `POST /api/v1/auth/providers/chatgpt/login/device/complete`
7. `POST /api/v1/auth/providers/chatgpt/login/browser/start`
8. `GET /api/v1/auth/providers/chatgpt/login/browser/callback/{login_id}`
9. Optional compatibility/manual completion via `POST /api/v1/auth/providers/chatgpt/login/browser/complete`
10. Optional explicit token handoff via `POST /api/v1/auth/providers/chatgpt/import`

Device flow may remain a two-step start/complete operation.

Browser flow should prefer:

1. host/client calls `login/browser/start`
2. client opens the returned `auth_url`
3. provider redirects back to the host-owned callback endpoint
4. host completes token exchange and persistence
5. client observes success/failure via auth events or status polling

`login/browser/complete` may continue to exist as a compatibility/manual completion path, but
clients should not need to receive and relay OAuth `code/state` when the host owns the callback.

## Account / Workspace Binding

ChatGPT-authenticated requests may need both:

1. a bearer token
2. a resolved ChatGPT account or workspace identity

Normative behavior:

1. Account/workspace identity is auth metadata, not prompt metadata.
2. If the provider requires account identity and none is available, the request must fail with a first-class auth error.
3. If a launch or host policy constrains the allowed workspace/account and the resolved login state does not match, the request must fail before model execution.

## Refresh and Recovery

ChatGPT auth support must include managed refresh behavior.

Minimum contract:

1. Runtime/provider layer may perform proactive refresh before request dispatch.
2. On an auth failure that indicates expired or invalid bearer state, the provider path may perform one managed refresh-and-retry cycle.
3. Repeated auth failure must surface as a first-class auth error, not as a generic transport error.

## Error Contract

ChatGPT-specific auth failures must be distinguishable from generic provider failures.

At minimum, the contract should separate:

1. not logged in
2. token expired / refresh required
3. refresh failed
4. workspace/account mismatch
5. unauthorized after refresh

This error family belongs to the provider/auth layer, not to the kernel turn-state contract.

## Relationship to Compaction

Compaction remains governed by [`compaction_contract.md`](./compaction_contract.md), not by provider login semantics.

Normative boundary:

1. Alan's compaction contract is provider-agnostic.
2. A provider-specific auxiliary endpoint such as remote `/responses/compact` may be used as an optimization, but it is not part of the ChatGPT auth contract.
3. ChatGPT authentication must not require Alan to adopt provider-specific compaction endpoints as a kernel dependency.

## Relationship to the Reference Coding Agent

The first real consumer of this provider/auth surface is the reference coding agent:

1. `#60` defines the provider/auth half.
2. `#10` defines the product-layer coding-agent half.
3. They should be developed in lockstep, but remain separate contracts.

## First-Phase Acceptance

For the experimental local path, the contract is satisfied when:

1. `openai_*` providers remain API Platform only.
2. A separate `chatgpt` provider can issue Responses-compatible requests.
3. Local managed login exists for ChatGPT auth.
4. ChatGPT account/workspace auth context is bridged into requests without leaking into prompt state.
5. Auth/account failures are first-class and distinguishable.
6. The reference coding agent can select this provider path without special-casing the kernel.

For the host-control-plane follow-on, the contract is additionally satisfied when:

1. Daemon/app-server clients can inspect ChatGPT auth status without shelling out to `alan auth`.
2. Login progress and account updates can be observed through a host event stream or replayable
   event surface.
3. Device flow can be initiated and completed through explicit host APIs, and browser flow can be
   initiated through host APIs and completed through a host-owned callback path.
4. Optional external token handoff, if enabled, remains explicit and policy-bounded.
5. The host path still reuses the same managed auth core as the local CLI flow.

## Explicit Non-Goals

This contract does not require:

1. provider-specific compaction endpoint adoption
2. collapsing API Platform and ChatGPT auth into one provider name
3. moving auth state into `agent.toml`, Tape, or the kernel session contract
