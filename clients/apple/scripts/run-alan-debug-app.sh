#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
DERIVED_DATA="$REPO_ROOT/target/xcode-derived"
APP_BUNDLE="$DERIVED_DATA/Build/Products/Debug/Alan.app"

wait_for_alan_to_exit() {
    local timeout_ticks=100

    for _ in $(seq 1 "$timeout_ticks"); do
        if ! pgrep -x Alan >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.1
    done

    printf 'error: timed out waiting for the previous Alan app process to exit\n' >&2
    return 1
}

if pgrep -x Alan >/dev/null 2>&1; then
    pkill -x Alan || true
    wait_for_alan_to_exit
fi

xcodebuild \
    -project "$REPO_ROOT/clients/apple/AlanNative.xcodeproj" \
    -scheme AlanNative \
    -configuration Debug \
    -destination platform=macOS \
    -derivedDataPath "$DERIVED_DATA" \
    build

wait_for_alan_to_exit
/usr/bin/open -n "$APP_BUNDLE"
