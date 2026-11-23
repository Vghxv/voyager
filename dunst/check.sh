#!/usr/bin/env bash
# Check the installed dunst theme. Read-only; ./doctor.sh runs this.
# Docs: dunst/dunst.md#if-it-breaks

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DIR="$HOME/.config/dunst"
FIX="./dunst/install.sh"

[[ -e "$VOYAGER_CONF/theme.d/dunst" || -f "$DIR/dunstrc.d/90-voyager-theme.conf" ]] ||
  { info "not installed ($FIX)"; exit 0; }

check_deps_report "$SRC/deps" "dunst"
check_same "$SRC/dunstrc" "$DIR/dunstrc" "$FIX"
check_files "color themes" "$FIX" "$SRC/themes" "$DIR/themes" light.conf dark.conf
check_same "$SRC/theme-hook.sh" "$VOYAGER_CONF/theme.d/dunst" "$FIX"

mode=$(cat "$VOYAGER_CONF/theme" 2>/dev/null || echo light)
if cmp -s "$SRC/themes/$mode.conf" "$DIR/dunstrc.d/90-voyager-theme.conf"; then
  pass "colors match the saved mode ($mode)"
else
  fail "colors do not match the saved mode ($mode)" "~/.config/scripts/voyager-theme apply"
fi

if pgrep -x dunst >/dev/null; then pass "dunst is running"
else info "dunst is not running (it starts on the next notification)"; fi

run_checks
