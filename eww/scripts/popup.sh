#!/usr/bin/env bash
# popup.sh <window> [monitor]: toggle a bar popup, with popup_closer (popup.yuck)
# under it so a click outside closes it. EWW: the eww binary (the bar passes its own).
eww=("${EWW:-eww}" --config "${0%/scripts/*}")
was_open=$("${eww[@]}" active-windows | grep "^$1:")
"${eww[@]}" close sound power_menu popup_closer 2>/dev/null
[[ -z "$was_open" ]] && "${eww[@]}" open popup_closer ${2:+--screen "$2"} && "${eww[@]}" open "$1" ${2:+--screen "$2"}
