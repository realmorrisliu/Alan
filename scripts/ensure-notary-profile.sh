#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=scripts/release-env.sh
source "$SCRIPT_DIR/release-env.sh"

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        fail "required command '$1' was not found"
    fi
}

profile="${ALAN_NOTARY_KEYCHAIN_PROFILE:-}"

require_command xcrun

if [[ -z "$profile" ]]; then
    fail "ALAN_NOTARY_KEYCHAIN_PROFILE is required for automated notarization setup"
fi

if xcrun notarytool history \
    --keychain-profile "$profile" \
    --output-format json \
    --no-progress >/dev/null 2>&1; then
    printf 'Notary keychain profile is available: %s\n' "$profile"
    exit 0
fi

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_TEAM_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]]; then
    printf 'Creating or updating notary keychain profile from Apple ID credentials: %s\n' "$profile"
    xcrun notarytool store-credentials "$profile" \
        --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_SPECIFIC_PASSWORD" \
        --validate
    exit 0
fi

fail "notary keychain profile '$profile' is not available, and APPLE_ID, APPLE_TEAM_ID, and APPLE_APP_SPECIFIC_PASSWORD are not all set in the release env"
