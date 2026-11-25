#!/usr/bin/env bash
# voyager-theme hook for the eww bar: restyle it in place.
#
#   theme-hook.sh light|dark
#
# Installed to ~/.config/voyager/theme.d/eww. Only sets the `theme` variable;
# eww.scss holds both palettes, so nothing reloads. A daemon that is not
# running gets the saved mode from bars.sh when it starts.

set -uo pipefail

case "${1:-}" in light|dark) ;; *) echo "usage: theme-hook.sh light|dark" >&2; exit 2 ;; esac

EWW="${EWW:-eww}"
"$EWW" ping >/dev/null 2>&1 || exit 0
"$EWW" update theme="$1"
