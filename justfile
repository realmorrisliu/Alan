# Alan - Development Tasks

# List available commands
default:
    @just --list

# Run tests
test:
    cargo test --workspace

# Check code (format + lint + test)
check: fmt lint test
    @echo "✅ All checks passed"

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run clippy
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Show coverage summary in terminal
coverage:
    cargo llvm-cov --workspace --summary-only

# Show detailed coverage with uncovered lines
coverage-detail:
    cargo llvm-cov --workspace

# Generate HTML coverage report (target/coverage/html)
coverage-html:
    cargo llvm-cov --workspace --html --output-dir target/coverage

# Run the agent daemon
serve:
    cargo run -p alan -- daemon start

# Build release
build:
    cargo build --release

# Install alan to ~/.alan/bin
install:
    ./scripts/install.sh

# Uninstall alan from ~/.alan/bin
uninstall:
    rm -rf "$HOME/.alan/bin/alan" "$HOME/.alan/bin/alan-tui" "$HOME/.alan/bin/alan-tui.js"
    echo "Alan uninstalled from ~/.alan/bin/"

# Clean artifacts
clean:
    cargo clean
    rm -rf target/

# Mock smoke tests (CI safe, no LLM needed)
smoke:
    cargo test -p alan --test smoke_test -- --nocapture

# End-to-end smoke test (needs ~/.alan LLM config)
smoke-e2e:
    bash scripts/smoke-e2e.sh

# Run autonomy harness scenarios (all)
harness-autonomy:
    bash scripts/harness/run_autonomy_suite.sh

# Run only CI-blocking autonomy harness scenarios
harness-autonomy-ci:
    bash scripts/harness/run_autonomy_suite.sh --ci-blocking

# Run self-eval profile regression in local mode
self-eval:
    bash scripts/harness/run_self_eval_suite.sh --mode local

# Run self-eval profile regression in CI gate mode
self-eval-ci:
    bash scripts/harness/run_self_eval_suite.sh --mode ci

# Run self-eval profile regression in nightly mode
self-eval-nightly:
    bash scripts/harness/run_self_eval_suite.sh --mode nightly

# Run coding reference smoke loop
coding-reference-smoke:
    bash scripts/reference/run_coding_reference_smoke.sh --mode local

# Run coding reference harness scenarios (all)
harness-coding-reference:
    bash scripts/harness/run_coding_reference_suite.sh

# Run only CI-blocking coding reference harness scenarios
harness-coding-reference-ci:
    bash scripts/harness/run_coding_reference_suite.sh --ci-blocking

# Run compaction harness scenarios (all)
harness-compaction:
    bash scripts/harness/run_compaction_suite.sh

# Run only CI-blocking compaction harness scenarios
harness-compaction-ci:
    bash scripts/harness/run_compaction_suite.sh --ci-blocking

# Coding agent verification loop (run after code changes)
verify: fmt lint test smoke
    @echo "✅ Core flows verified"

# Full verification including real LLM
verify-full: verify smoke-e2e
    @echo "✅ Full verification passed (including E2E)"
