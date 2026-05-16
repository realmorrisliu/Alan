#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DERIVED_DATA="${ALAN_XCODE_DERIVED_DATA:-$REPO_ROOT/target/xcode-derived}"
APP_BUNDLE="${1:-$DERIVED_DATA/Build/Products/Release/alan.app}"
MANIFEST="$APP_BUNDLE/Contents/Resources/alan-package-manifest.json"
ALAN_BIN="$APP_BUNDLE/Contents/Resources/bin/alan"
TUI_BIN="$APP_BUNDLE/Contents/Resources/bin/alan-tui"

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

require_executable() {
    local path="$1"
    [[ -x "$path" ]] || fail "expected executable at $path"
}

require_developer_id_signature() {
    local path="$1"
    local details

    details="$(codesign -dv --verbose=4 "$path" 2>&1)" || fail "codesign could not inspect $path"
    if printf '%s\n' "$details" | grep -q 'Signature=adhoc'; then
        fail "ad-hoc signature is not allowed for $path"
    fi
    if ! printf '%s\n' "$details" | grep -q 'Authority=Developer ID Application'; then
        fail "Developer ID Application signature is required for $path"
    fi
}

require_entitlement() {
    local path="$1"
    local entitlement="$2"
    local entitlements

    entitlements="$(codesign -d --entitlements :- "$path" 2>/dev/null)" ||
        fail "codesign could not inspect entitlements for $path"
    if ! printf '%s\n' "$entitlements" | grep -q "$entitlement"; then
        fail "required entitlement $entitlement is missing from $path"
    fi
}

manifest_value() {
    local key="$1"

    sed -nE "s/.*\"$key\": \"([^\"]+)\".*/\\1/p" "$MANIFEST" | head -n 1
}

manifest_binary_sha256() {
    local binary="$1"

    awk -v binary="\"$binary\"" '
        $0 ~ binary "[[:space:]]*:" {
            in_binary = 1
            next
        }
        in_binary && /"sha256"[[:space:]]*:/ {
            value = $0
            sub(/^.*"sha256"[[:space:]]*:[[:space:]]*"/, "", value)
            sub(/".*$/, "", value)
            print value
            exit
        }
        in_binary && /^[[:space:]]*}/ {
            in_binary = 0
        }
    ' "$MANIFEST"
}

require_manifest_checksum() {
    local binary="$1"
    local path="$2"
    local expected
    local actual

    expected="$(manifest_binary_sha256 "$binary")"
    [[ -n "$expected" ]] || fail "manifest does not record sha256 for $binary"
    actual="$(shasum -a 256 "$path" | awk '{print $1}')"
    if [[ "$actual" != "$expected" ]]; then
        fail "manifest sha256 for $binary does not match embedded binary"
    fi
}

[[ -d "$APP_BUNDLE" ]] || fail "app bundle not found: $APP_BUNDLE"
require_executable "$APP_BUNDLE/Contents/MacOS/alan"
require_executable "$ALAN_BIN"
require_executable "$TUI_BIN"
[[ -f "$MANIFEST" ]] || fail "package manifest not found: $MANIFEST"

grep -q '"path": "Contents/Resources/bin/alan"' "$MANIFEST" ||
    fail "manifest does not record embedded alan path"
grep -q '"path": "Contents/Resources/bin/alan-tui"' "$MANIFEST" ||
    fail "manifest does not record embedded alan-tui path"

manifest_version="$(manifest_value "version")"
repo_version="$(awk -F '"' '/^version = / { print $2; exit }' "$REPO_ROOT/Cargo.toml")"
[[ -n "$manifest_version" ]] || fail "manifest does not record package version"
if [[ "$manifest_version" != "$repo_version" ]]; then
    fail "manifest version $manifest_version does not match Cargo.toml version $repo_version"
fi
require_manifest_checksum "alan" "$ALAN_BIN"
require_manifest_checksum "alan-tui" "$TUI_BIN"

require_developer_id_signature "$ALAN_BIN"
require_developer_id_signature "$TUI_BIN"
require_entitlement "$TUI_BIN" "com.apple.security.cs.allow-jit"
require_developer_id_signature "$APP_BUNDLE"
codesign --verify --strict --verbose=2 "$APP_BUNDLE" >/dev/null

if [[ "${ALAN_VALIDATE_NOTARIZATION:-0}" == "1" ]]; then
    xcrun stapler validate "$APP_BUNDLE" >/dev/null
fi

printf 'Release app validation passed: %s\n' "$APP_BUNDLE"
