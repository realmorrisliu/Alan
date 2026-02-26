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
    rm -rf "$HOME/.alan/bin/alan" "$HOME/.alan/bin/alan-tui.js"
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

# Coding agent verification loop (run after code changes)
verify: fmt lint test smoke
    @echo "✅ Core flows verified"

# Full verification including real LLM
verify-full: verify smoke-e2e
    @echo "✅ Full verification passed (including E2E)"
