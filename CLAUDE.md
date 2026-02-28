## Verification Loop

After each Rust code change under `crates/`, run `just verify` to confirm the core flow is healthy.
If `~/.alan` LLM config is available on your machine, run `just verify-full` for end-to-end validation.
If verification fails, inspect logs, identify the issue, and fix it before proceeding.
