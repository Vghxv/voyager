#!/usr/bin/env bash
# voyager-theme hook for dunst: swap the color drop-in and reload.
#
#   theme-hook.sh light|dark
#
# Installed to ~/.config/voyager/theme.d/dunst. Copies themes/<mode>.conf to
# dunstrc.d/90-voyager-theme.conf, which dunst reads after dunstrc.

set -uo pipefail

case "${1:-}" in light|dark) ;; *) echo "usage: theme-hook.sh light|dark" >&2; exit 2 ;; esac

DIR="$HOME/.config/dunst"
src="$DIR/themes/$1.conf"
[[ -f "$src" ]] || { echo "$src is missing — re-run dunst/install.sh"; exit 1; }
install -D -m 644 "$src" "$DIR/dunstrc.d/90-voyager-theme.conf" || exit 1
pgrep -x dunst >/dev/null || exit 0   # not running: starts with these colors
dunstctl reload || { echo "dunstctl reload failed — restart dunst: pkill -x dunst"; exit 1; }
