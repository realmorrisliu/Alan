#!/usr/bin/env bash
set -e

ALAN_HOME="${ALAN_HOME:-$HOME/.alan}"
BIN_DIR="$ALAN_HOME/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DAEMON_URL="${ALAN_AGENTD_URL:-http://127.0.0.1:8090}"
DAEMON_WAS_RUNNING=0

if command -v curl >/dev/null 2>&1; then
  if curl -fsS --max-time 2 "$DAEMON_URL/health" >/dev/null 2>&1; then
    DAEMON_WAS_RUNNING=1
  fi
fi

echo "Building Alan..."

# 1. Build Rust binary
cd "$PROJECT_ROOT"
cargo build --release -p alan

# 2. Build standalone TUI executable
echo "Building TUI..."
mkdir -p "$BIN_DIR"
cd "$PROJECT_ROOT/clients/tui"
bun install --frozen-lockfile 2>/dev/null || bun install
NODE_PATH="$PROJECT_ROOT/clients/tui/.shims" bun build src/index.tsx \
  --outfile="$BIN_DIR/alan-tui" \
  --target=bun \
  --compile
chmod +x "$BIN_DIR/alan-tui"
# Clean up old bundle from previous install to avoid stale fallback behavior.
rm -f "$BIN_DIR/alan-tui.js"

# 3. Install binary
cp "$PROJECT_ROOT/target/release/alan" "$BIN_DIR/"
chmod +x "$BIN_DIR/alan"
# Re-sign binary to clear macOS kernel code-signature cache.
# Without this, overwriting a previously-signed binary at the same path
# causes SIGKILL on launch (code-signature hash mismatch).
codesign -f -s - "$BIN_DIR/alan" 2>/dev/null || true
codesign -f -s - "$BIN_DIR/alan-tui" 2>/dev/null || true

# 4. Restart running daemon so the new binary takes effect immediately.
if [ "$DAEMON_WAS_RUNNING" -eq 1 ]; then
  echo "Restarting daemon to apply the new version..."
  if "$BIN_DIR/alan" daemon stop >/dev/null 2>&1 && "$BIN_DIR/alan" daemon start >/dev/null 2>&1; then
    echo "Daemon restarted."
  else
    echo "⚠️  Failed to restart daemon automatically."
    echo "   Run manually: $BIN_DIR/alan daemon stop && $BIN_DIR/alan daemon start"
  fi
fi

# 5. Initialize root workspace if needed
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
