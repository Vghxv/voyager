#!/usr/bin/env bash
# Toggle a bar popup, with popup_closer (popup.yuck) under it so a click outside
# closes it. EWW: the eww binary (the bar passes its own).
#
#   popup.sh <window> [monitor]   toggle <window> on <monitor>
#   popup.sh close                close every popup

source "$(dirname "$0")/lib.sh"
eww=("$EWW" --config "$(dirname "$(dirname "$(readlink -f "$0")")")")

[[ -n "${1:-}" ]] || { echo "usage: $0 <window> [monitor] | close" >&2; exit 1; }
was_open=$("${eww[@]}" active-windows | awk -F: -v w="$1" '$1 == w')
"${eww[@]}" close "${POPUPS[@]}" 2>/dev/null
[[ "$1" == close || -n "$was_open" ]] && exit 0

screen=()
[[ -n "${2:-}" ]] && screen=(--screen "$2")
"${eww[@]}" open popup_closer "${screen[@]}" && "${eww[@]}" open "$1" "${screen[@]}"
