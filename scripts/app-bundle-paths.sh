#!/usr/bin/env bash

alan_path_exists() {
    local path="$1"

    [[ -e "$path" || -L "$path" ]]
}

alan_is_distinct_existing_path() {
    local candidate="$1"
    local reference="$2"

    alan_path_exists "$candidate" || return 1
    alan_path_exists "$reference" || return 0

    [[ ! "$candidate" -ef "$reference" ]]
}
