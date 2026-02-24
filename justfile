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
    cargo run -p alan-agentd

# Build release
build:
    cargo build --release

# Clean artifacts
clean:
    cargo clean
    rm -rf target/
