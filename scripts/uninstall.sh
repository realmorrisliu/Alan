#!/usr/bin/env bash
set -euo pipefail

APP_INSTALL_DIR="${ALAN_APP_INSTALL_DIR:-$HOME/Applications}"
APP_TARGET="$APP_INSTALL_DIR/alan.app"
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
        *"/alan.app/Contents/Resources/bin/$tool")
            rm -f "$path"
            ;;
    esac
}

remove_alan_link "alan"
remove_alan_link "alan-tui"
rm -rf "$APP_TARGET"

printf 'alan app and PATH symlinks were removed when owned by this install.\n'
printf 'User data under ~/.alan was left intact.\n'
