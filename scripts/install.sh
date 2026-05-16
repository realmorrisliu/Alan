#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# shellcheck source=scripts/release-env.sh
source "$SCRIPT_DIR/release-env.sh"

DERIVED_DATA="${ALAN_XCODE_DERIVED_DATA:-$PROJECT_ROOT/target/xcode-derived}"
APP_SOURCE="$DERIVED_DATA/Build/Products/Release/Alan.app"
APP_INSTALL_DIR="${ALAN_APP_INSTALL_DIR:-$HOME/Applications}"
APP_TARGET="$APP_INSTALL_DIR/Alan.app"
LEGACY_APP_TARGET="$APP_INSTALL_DIR/alan.app"
CLI_INSTALL_DIR="${ALAN_CLI_INSTALL_DIR:-/usr/local/bin}"
APP_WAS_RUNNING=0

is_app_running() {
    pgrep -f "/Alan\\.app/Contents/MacOS/Alan" >/dev/null 2>&1 ||
        pgrep -f "/alan\\.app/Contents/MacOS/alan" >/dev/null 2>&1
}

is_alan_owned_link() {
    local path="$1"
    local target

    if [[ ! -L "$path" ]]; then
        return 1
    fi

    target="$(readlink "$path")"
    case "$target" in
        *"/Alan.app/Contents/Resources/bin/"*|*"/alan.app/Contents/Resources/bin/"*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_homebrew_prefix_target() {
    local target_dir="$1"
    local prefix
    local prefixes=()

    if command -v brew >/dev/null 2>&1; then
        prefix="$(brew --prefix 2>/dev/null || true)"
        if [[ -n "$prefix" ]]; then
            prefixes+=("$prefix")
        fi
    fi

    [[ -d /opt/homebrew ]] && prefixes+=("/opt/homebrew")
    [[ -d /usr/local/Homebrew ]] && prefixes+=("/usr/local")

    for prefix in "${prefixes[@]}"; do
        case "$target_dir/" in
            "$prefix/"*)
                return 0
                ;;
        esac
    done

    return 1
}

has_homebrew_managed_tool_links() {
    local prefix
    local tool
    local link
    local target
    local prefixes=()

    if command -v brew >/dev/null 2>&1; then
        prefix="$(brew --prefix 2>/dev/null || true)"
        if [[ -n "$prefix" ]]; then
            prefixes+=("$prefix")
        fi
    fi

    [[ -d /opt/homebrew ]] && prefixes+=("/opt/homebrew")
    [[ -d /usr/local/Homebrew ]] && prefixes+=("/usr/local")

    for prefix in "${prefixes[@]}"; do
        for tool in alan alan-tui; do
            link="$prefix/bin/$tool"
            if [[ ! -L "$link" ]]; then
                continue
            fi
            target="$(readlink "$link")"
            case "$target" in
                *"/Alan.app/Contents/Resources/bin/$tool"|*"/alan.app/Contents/Resources/bin/$tool")
                    printf '%s\n' "$link"
                    return 0
                    ;;
            esac
        done
    done

    return 1
}

link_tool() {
    local tool="$1"
    local source="$APP_TARGET/Contents/Resources/bin/$tool"
    local target="$CLI_INSTALL_DIR/$tool"

    if [[ ! -x "$source" ]]; then
        printf 'error: embedded tool is missing or not executable: %s\n' "$source" >&2
        exit 1
    fi

    if [[ -e "$target" || -L "$target" ]]; then
        if ! is_alan_owned_link "$target"; then
            printf 'error: refusing to overwrite non-alan command at %s\n' "$target" >&2
            printf '       set ALAN_CLI_INSTALL_DIR to a different PATH directory or remove the conflicting file manually\n' >&2
            exit 1
        fi
        if [[ "$(readlink "$target")" == "$source" ]]; then
            return
        fi
        rm -f "$target"
    fi

    ln -s "$source" "$target"
}

if is_app_running; then
    APP_WAS_RUNNING=1
fi

"$SCRIPT_DIR/assemble-release-app.sh"

if [[ ! -d "$APP_SOURCE" ]]; then
    printf 'error: release assembly did not produce %s\n' "$APP_SOURCE" >&2
    exit 1
fi

printf 'Installing Alan.app to %s...\n' "$APP_TARGET"
mkdir -p "$APP_INSTALL_DIR"
rm -rf "$APP_TARGET"
ditto "$APP_SOURCE" "$APP_TARGET"
if [[ "$LEGACY_APP_TARGET" != "$APP_TARGET" && -d "$LEGACY_APP_TARGET" ]]; then
    printf 'Removing legacy lowercase app bundle at %s...\n' "$LEGACY_APP_TARGET"
    rm -rf "$LEGACY_APP_TARGET"
fi

printf 'Linking CLI and TUI into %s...\n' "$CLI_INSTALL_DIR"
if is_homebrew_prefix_target "$CLI_INSTALL_DIR"; then
    printf 'error: %s is inside a Homebrew prefix.\n' "$CLI_INSTALL_DIR" >&2
    printf '       use the Homebrew cask for Homebrew-managed links, or set ALAN_CLI_INSTALL_DIR to a non-Homebrew PATH directory.\n' >&2
    exit 1
fi
if homebrew_link="$(has_homebrew_managed_tool_links)"; then
    printf 'error: Homebrew already manages alan command-line links at %s\n' "$homebrew_link" >&2
    printf '       use the Homebrew cask to update alan, or remove the Homebrew links before creating direct-install symlinks.\n' >&2
    exit 1
fi
mkdir -p "$CLI_INSTALL_DIR"
link_tool "alan"
link_tool "alan-tui"

printf '\nAlan installed:\n'
printf '  app: %s\n' "$APP_TARGET"
printf '  cli: %s/alan -> %s/Contents/Resources/bin/alan\n' "$CLI_INSTALL_DIR" "$APP_TARGET"
printf '  tui: %s/alan-tui -> %s/Contents/Resources/bin/alan-tui\n' "$CLI_INSTALL_DIR" "$APP_TARGET"

if [[ "$APP_WAS_RUNNING" -eq 1 ]]; then
    printf '\nAlan.app was running during install. It was not stopped or relaunched; restart it manually to use the newly installed app.\n'
fi

printf '\nEnsure %s is on PATH if you want shell access.\n' "$CLI_INSTALL_DIR"
