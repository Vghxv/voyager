#!/usr/bin/env bash
# Check the installed hyprlock theme. Read-only; ./doctor.sh runs this.
# Docs: hyprlock/hyprlock.md#if-it-breaks

set -uo pipefail   # no -e: every check runs, failures are reported

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"
DEST="$HYPR_VOYAGER"
FIX="./hyprlock/install.sh"

[[ -f "$DEST/next-frame.sh" ]] || not_installed "$FIX"

check_deps_report "$SRC/deps" "hyprlock"
check_same "$SRC/hyprlock.conf" "$HOME/.config/hypr/hyprlock.conf" "$FIX   (or --restore for your own)"
cd "$SRC" || exit 1
check_files "frames and probe" "$FIX" "$SRC" "$DEST" stars-*.png probe.png next-frame.sh
extra=()
for f in "$DEST"/stars-*.png; do
  [[ -e "$f" && ! -e "$SRC/${f##*/}" ]] && extra+=("${f##*/}")
done
(( ${#extra[@]} )) && fail "leftover frames in $DEST: ${extra[*]}" "$FIX"

frame=$("$DEST/next-frame.sh")
if [[ -f "$frame" ]]; then pass "next-frame.sh picks an existing frame"
else fail "next-frame.sh points at missing $frame" "$FIX"; fi

run_checks
