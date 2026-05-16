#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/app-bundle-paths.sh
source "$SCRIPT_DIR/app-bundle-paths.sh"

APP_INSTALL_DIR="${ALAN_APP_INSTALL_DIR:-$HOME/Applications}"
APP_TARGET="$APP_INSTALL_DIR/Alan.app"
LEGACY_APP_TARGET="$APP_INSTALL_DIR/alan.app"
CLI_INSTALL_DIR="${ALAN_CLI_INSTALL_DIR:-/usr/local/bin}"

remove_alan_link() {
    local tool="$1"
    local path="$CLI_INSTALL_DIR/$tool"
    local target

    if [[ ! -L "$path" ]]; then
        return
    fi

    target="$(readlink "$path")"
    case "$target" in
        *"/Alan.app/Contents/Resources/bin/$tool"|*"/alan.app/Contents/Resources/bin/$tool")
            rm -f "$path"
            ;;
    esac
}

remove_alan_link "alan"
remove_alan_link "alan-tui"
rm -rf "$APP_TARGET"
if alan_is_distinct_existing_path "$LEGACY_APP_TARGET" "$APP_TARGET"; then
    rm -rf "$LEGACY_APP_TARGET"
fi

printf 'Alan app and PATH symlinks were removed when owned by this install.\n'
printf 'User data under ~/.alan was left intact.\n'
