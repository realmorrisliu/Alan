#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

GHOSTTY_REPO="${ALAN_GHOSTTY_REPO:-$HOME/Developer/ghostty}"
CACHE_ROOT="${ALAN_GHOSTTY_CACHE_DIR:-$HOME/.cache/alan-shell/ghostty}"

OUTPUT_XCFRAMEWORK="$PROJECT_DIR/GhosttyKit.xcframework"
OUTPUT_RESOURCES="$PROJECT_DIR/ghostty-resources"
OUTPUT_TERMINFO="$PROJECT_DIR/ghostty-terminfo"

CACHE_KEY=""
CACHE_DIR=""
CACHE_XCFRAMEWORK=""
CACHE_RESOURCES=""
CACHE_TERMINFO=""

sync_tree() {
    local source="$1"
    local destination="$2"

    if [ ! -d "$source" ]; then
        printf 'error: source directory not found at %s\n' "$source" >&2
        exit 1
    fi

    rm -rf "$destination"
    mkdir -p "$destination"
    rsync -a --delete "$source"/ "$destination"/
}

sync_path() {
    local source="$1"
    local destination="$2"

    if [ -d "$source" ]; then
        sync_tree "$source" "$destination"
        return 0
    fi

    printf 'error: source directory not found at %s\n' "$source" >&2
    exit 1
}

link_output() {
    local source="$1"
    local destination="$2"

    rm -rf "$destination"
    ln -sfn "$source" "$destination"
}

require_metal_toolchain() {
    if xcrun -sdk macosx --find metal >/dev/null 2>&1; then
        return 0
    fi

    printf 'error: Metal Toolchain is required to build GhosttyKit.xcframework\n' >&2
    printf 'hint: run `xcodebuild -downloadComponent MetalToolchain`\n' >&2
    exit 1
}

find_existing_framework() {
    local path
    for path in \
        "${ALAN_GHOSTTYKIT_PATH:-}" \
        "$GHOSTTY_REPO/macos/GhosttyKit.xcframework"; do
        if [ -d "$path" ]; then
            printf '%s\n' "$path"
            return 0
        fi
    done
    return 1
}

resolve_cache_key() {
    if [ -n "${ALAN_GHOSTTY_CACHE_KEY:-}" ]; then
        printf '%s\n' "$ALAN_GHOSTTY_CACHE_KEY"
        return 0
    fi

    if [ -d "$GHOSTTY_REPO" ] && git -C "$GHOSTTY_REPO" rev-parse --is-inside-work-tree >/dev/null 2>&1;
    then
        git -C "$GHOSTTY_REPO" rev-parse HEAD
        return 0
    fi

    printf '%s\n' "${ALAN_GHOSTTYKIT_PATH:-}|${ALAN_GHOSTTY_RESOURCES_DIR:-}|${ALAN_GHOSTTY_TERMINFO_DIR:-}" \
        | shasum -a 256 \
        | awk '{print "manual-" substr($1, 1, 16)}'
}

prepare_cache_paths() {
    CACHE_KEY="$(resolve_cache_key)"
    CACHE_DIR="$CACHE_ROOT/$CACHE_KEY"
    CACHE_XCFRAMEWORK="$CACHE_DIR/GhosttyKit.xcframework"
    CACHE_RESOURCES="$CACHE_DIR/ghostty-resources"
    CACHE_TERMINFO="$CACHE_DIR/ghostty-terminfo"

    mkdir -p "$CACHE_DIR"
}

ensure_ghosttykit() {
    local resolved=""
    if resolved="$(find_existing_framework)"; then
        printf '==> Reusing GhosttyKit source at %s\n' "$resolved"
    else
        if ! command -v zig >/dev/null 2>&1; then
            printf 'error: zig is required to build GhosttyKit.xcframework\n' >&2
            exit 1
        fi

        require_metal_toolchain

        if [ ! -d "$GHOSTTY_REPO" ]; then
            printf 'error: Ghostty repo not found at %s\n' "$GHOSTTY_REPO" >&2
            exit 1
        fi

        printf '==> Building GhosttyKit.xcframework from %s\n' "$GHOSTTY_REPO"
        (
            cd "$GHOSTTY_REPO"
            zig build -Demit-xcframework=true -Dxcframework-target=universal -Doptimize=ReleaseFast
        )

        resolved="$GHOSTTY_REPO/macos/GhosttyKit.xcframework"
        if [ ! -d "$resolved" ]; then
            printf 'error: expected GhosttyKit.xcframework at %s\n' "$resolved" >&2
            exit 1
        fi
    fi

    printf '==> Syncing %s -> %s\n' "$resolved" "$CACHE_XCFRAMEWORK"
    sync_path "$resolved" "$CACHE_XCFRAMEWORK"

    printf '==> Linking %s -> %s\n' "$OUTPUT_XCFRAMEWORK" "$CACHE_XCFRAMEWORK"
    link_output "$CACHE_XCFRAMEWORK" "$OUTPUT_XCFRAMEWORK"
}

ensure_resources() {
    local candidate
    for candidate in \
        "${ALAN_GHOSTTY_RESOURCES_DIR:-}" \
        "$GHOSTTY_REPO/zig-out/share/ghostty"; do
        if [ -d "$candidate" ]; then
            printf '==> Syncing %s -> %s\n' "$candidate" "$CACHE_RESOURCES"
            sync_path "$candidate" "$CACHE_RESOURCES"
            printf '==> Linking %s -> %s\n' "$OUTPUT_RESOURCES" "$CACHE_RESOURCES"
            link_output "$CACHE_RESOURCES" "$OUTPUT_RESOURCES"
            return 0
        fi
    done

    printf 'warning: no Ghostty resources directory found; continuing without %s\n' "$OUTPUT_RESOURCES" >&2
}

ensure_terminfo() {
    local candidate
    for candidate in \
        "${ALAN_GHOSTTY_TERMINFO_DIR:-}" \
        "$GHOSTTY_REPO/zig-out/share/terminfo"; do
        if [ -d "$candidate" ]; then
            printf '==> Syncing %s -> %s\n' "$candidate" "$CACHE_TERMINFO"
            sync_path "$candidate" "$CACHE_TERMINFO"
            printf '==> Linking %s -> %s\n' "$OUTPUT_TERMINFO" "$CACHE_TERMINFO"
            link_output "$CACHE_TERMINFO" "$OUTPUT_TERMINFO"
            return 0
        fi
    done

    printf 'warning: no Ghostty terminfo directory found; continuing without %s\n' "$OUTPUT_TERMINFO" >&2
}

check_artifacts() {
    local missing=0
    local path
    for path in "$OUTPUT_XCFRAMEWORK" "$OUTPUT_RESOURCES" "$OUTPUT_TERMINFO"; do
        if [ -d "$path" ]; then
            printf 'ok: %s\n' "$path"
        else
            printf 'missing: %s\n' "$path" >&2
            missing=1
        fi
    done

    if [ "$missing" -ne 0 ]; then
        printf '\nRun %s to prepare the local Ghostty artifacts.\n' "$0" >&2
        exit 1
    fi
}

case "${1:-}" in
    --check)
        check_artifacts
        exit 0
        ;;
    "")
        ;;
    *)
        printf 'usage: %s [--check]\n' "$0" >&2
        exit 2
        ;;
esac

prepare_cache_paths
ensure_ghosttykit
ensure_resources
ensure_terminfo

printf '\nReady.\n'
printf 'Ghostty artifacts are cached outside the repo at %s.\n' "$CACHE_DIR"
printf 'Ignored developer links under clients/apple/ now point at that cache.\n'
