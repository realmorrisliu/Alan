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

# Install alan to ~/.alan/bin
install: build
    #!/usr/bin/env bash
    set -e
    echo "Installing Alan to ~/.alan/bin/"
    mkdir -p "$HOME/.alan/bin"
    cp target/release/agentd "$HOME/.alan/bin/"
    chmod +x "$HOME/.alan/bin/agentd"
    
    # Build and install TUI
    cd clients/tui
    bun install
    
    # Use wrapper script approach for better compatibility
    echo "Building TUI bundle..."
    bun build src/index.tsx --outfile="$HOME/.alan/bin/alan.js" --target=bun --external react-devtools-core
    
    # Create wrapper script
    echo '#!/bin/bash' > "$HOME/.alan/bin/alan"
    echo 'export ALAN_AGENTD_PATH="${ALAN_AGENTD_PATH:-$HOME/.alan/bin/agentd}"' >> "$HOME/.alan/bin/alan"
    echo 'cd "$HOME/.alan/bin" && exec bun run "$HOME/.alan/bin/alan.js" "$@"' >> "$HOME/.alan/bin/alan"
    chmod +x "$HOME/.alan/bin/alan"
    
    echo ""
    echo "✅ Alan installed to ~/.alan/bin/"
    echo ""
    echo "Next steps:"
    echo "1. Add to your shell config:"
    echo "   fish: set -gx PATH \$HOME/.alan/bin \$PATH"
    echo "   bash/zsh: export PATH=\"\$HOME/.alan/bin:\$PATH\""
    echo ""
    echo "2. Run: alan"
    echo "   First run will start setup wizard"

# Uninstall alan from ~/.alan/bin
uninstall:
    rm -rf "$HOME/.alan/bin/agentd" "$HOME/.alan/bin/alan" "$HOME/.alan/bin/alan.js"
    echo "Alan uninstalled from ~/.alan/bin/"

# Clean artifacts
clean:
    cargo clean
    rm -rf target/
