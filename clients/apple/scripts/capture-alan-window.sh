#!/bin/zsh
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
SOURCE="$SCRIPT_DIR/capture-alan-window.swift"
CACHE_DIR="${TMPDIR:-/tmp}/alan-shell-tools"
BINARY="$CACHE_DIR/capture-alan-window"

mkdir -p "$CACHE_DIR"

if [[ ! -x "$BINARY" || "$SOURCE" -nt "$BINARY" ]]; then
  xcrun swiftc \
    -O \
    -sdk "$(xcrun --sdk macosx --show-sdk-path)" \
    "$SOURCE" \
    -o "$BINARY"
fi

exec "$BINARY" "$@"
