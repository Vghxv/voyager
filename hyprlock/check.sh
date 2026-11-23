#!/usr/bin/env bash
# Check the installed hyprlock theme. Read-only; ./doctor.sh runs this.
# Docs: hyprlock/hyprlock.md#if-it-breaks

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DEST="$HOME/.config/hypr/voyager"
FIX="./hyprlock/install.sh"

[[ -f "$DEST/next-frame.sh" ]] || { info "not installed ($FIX)"; exit 0; }

check_deps_report "$SRC/deps" "hyprlock"
check_same "$SRC/hyprlock.conf" "$HOME/.config/hypr/hyprlock.conf" "$FIX   (or --restore for your own)"
cd "$SRC" || exit 1
check_files "frames and probe" "$FIX" "$SRC" "$DEST" stars-*.png probe.png next-frame.sh
extra=$(comm -13 <(ls stars-*.png) <(cd "$DEST" && ls stars-*.png) | tr '\n' ' ')
[[ -z "$extra" ]] || fail "leftover frames in $DEST: $extra" "$FIX"

frame=$("$DEST/next-frame.sh")
[[ -f "$frame" ]] && pass "next-frame.sh picks an existing frame" || fail "next-frame.sh points at missing $frame" "$FIX"

run_checks
