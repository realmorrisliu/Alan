# Live Runtime Smoke

> Status: current implementation guide for opt-in runtime-level live validation.

## Purpose

This smoke layer validates Alan's real runtime turn path against a live
provider. It exists to catch bugs that the provider-only harness cannot see,
such as runtime-default request fields leaking into provider-specific request
shaping, or regressions in runtime-owned memory surfaces.

Unlike the provider harness, this layer drives:

1. runtime startup,
2. session event streaming,
3. live turn submission,
4. runtime-to-provider request shaping.

## Current Coverage

Current entry point:

```text
crates/alan/tests/live_runtime_smoke_test.rs
```

Current provider coverage:

1. managed `chatgpt`

This is intentionally narrow for now. It should expand only when a provider has
clear runtime-specific risk that provider-only adapter tests do not cover.

## Safety Model

This smoke is protected in two ways:

1. every test is marked `#[ignore]`, so normal `cargo test` does not run it;
2. the runner requires `ALAN_LIVE_PROVIDER_TESTS=1`.

## Required Environment

Required:

- `ALAN_LIVE_CHATGPT_AUTH_STORAGE_PATH`

Optional:

- `ALAN_LIVE_CHATGPT_BASE_URL`
- `ALAN_LIVE_CHATGPT_MODEL`
- `ALAN_LIVE_CHATGPT_ACCOUNT_ID`

## What It Asserts

The current managed `chatgpt` runtime smoke asserts that:

1. a runtime configured from live ChatGPT auth can start successfully;
2. a submitted user turn reaches `TurnCompleted`;
3. the turn emits assistant text containing the requested token;
4. stable cross-session preference memory can be persisted and recalled;
5. recent-work continuity survives restart through runtime-owned handoff
   surfaces;
6. the runtime does not emit a provider error during the turn.

This specifically covers the real runtime path that previously leaked the
default `temperature=0.3` field into the managed ChatGPT Responses request.

## Usage

Run via script:

```bash
ALAN_LIVE_PROVIDER_TESTS=1 \
bash scripts/live-runtime-smoke.sh
```

Run the single ChatGPT smoke directly:

```bash
ALAN_LIVE_PROVIDER_TESTS=1 \
ALAN_LIVE_CHATGPT_AUTH_STORAGE_PATH="$HOME/.alan/auth.json" \
cargo test -p alan --test live_runtime_smoke_test live_chatgpt_runtime_smoke -- --ignored --nocapture
```

Use this layer together with the provider harness:

1. `live_provider_harness` proves the adapter contract directly;
2. `live_runtime_smoke` proves the actual runtime turn path still respects that
   contract.
