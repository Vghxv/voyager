#!/usr/bin/env bash
# Check the installed hyprlock theme. Read-only; ./doctor.sh runs this.
# Docs: hyprlock/hyprlock.md#if-it-breaks

set -uo pipefail   # no -e: every check runs, failures are reported

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"
DEST="$HYPR_VOYAGER"
FIX="./hyprlock/install.sh"
CONF="$HOME/.config/hypr/hyprlock.conf"

[[ -f "$CONF" ]] && grep -q '^# voyager' "$CONF" || not_installed "$FIX"

check_deps_report "$SRC/deps" "hyprlock"
check_same "$SRC/hyprlock.conf" "$CONF" "$FIX   (or --restore for your own)"
check_files "probe files" "$FIX" "$SRC" "$DEST" probe.png

if [[ -s "$DEST/lock.png" ]]; then pass "lock.png (the background) is there"
else info "no $DEST/lock.png: the background is plain navy"; fi

leftover=("$DEST"/stars-*.png "$DEST"/next-frame.sh)
[[ -e "${leftover[0]}" || -e "${leftover[-1]}" ]] && fail "old twinkle frames left in $DEST" "$FIX"

run_checks
