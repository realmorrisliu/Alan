#!/usr/bin/env bash
set -e

ALAN_HOME="${ALAN_HOME:-$HOME/.alan}"
BIN_DIR="$ALAN_HOME/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building Alan..."

# 1. Build Rust binary
cd "$PROJECT_ROOT"
cargo build --release -p alan

# 2. Build TUI bundle
echo "Building TUI..."
cd "$PROJECT_ROOT/clients/tui"
bun install --frozen-lockfile 2>/dev/null || bun install
bun build src/index.tsx \
  --outfile="$BIN_DIR/alan-tui.js" \
  --target=bun \
  --external react-devtools-core

# 3. Install binary
mkdir -p "$BIN_DIR"
cp "$PROJECT_ROOT/target/release/alan" "$BIN_DIR/"
chmod +x "$BIN_DIR/alan"
# Re-sign binary to clear macOS kernel code-signature cache.
# Without this, overwriting a previously-signed binary at the same path
# causes SIGKILL on launch (code-signature hash mismatch).
codesign -f -s - "$BIN_DIR/alan" 2>/dev/null || true

# 4. Initialize root workspace if needed
if [ ! -d "$ALAN_HOME/context" ]; then
  "$BIN_DIR/alan" init --path "$ALAN_HOME" --name root --silent 2>/dev/null || true
fi

echo ""
echo "✅ Alan installed to $BIN_DIR"
echo ""
echo "Next steps:"
echo "1. Add to your shell config:"
echo "   fish: set -gx PATH $BIN_DIR \$PATH"
echo "   bash/zsh: export PATH=\"$BIN_DIR:\$PATH\""
echo ""
echo "2. Run: alan"
echo "   (starts interactive chat, auto-launches daemon)"
