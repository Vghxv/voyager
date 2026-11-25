#!/usr/bin/env bash
# Check the installed dunst theme. Read-only; ./doctor.sh runs this.
# Docs: dunst/dunst.md#if-it-breaks

set -uo pipefail   # no -e: every check runs, failures are reported

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"
DIR="$HOME/.config/dunst"
FIX="./dunst/install.sh"

DROPIN="$DIR/dunstrc.d/90-voyager-theme.conf"
HOOK="$THEME_HOOKS/dunst"

[[ -e "$HOOK" || -f "$DROPIN" ]] || not_installed "$FIX"

check_deps_report "$SRC/deps" "dunst"
check_same "$SRC/dunstrc" "$DIR/dunstrc" "$FIX"
check_files "color themes" "$FIX" "$SRC/themes" "$DIR/themes" light.conf dark.conf
check_same "$SRC/theme-hook.sh" "$HOOK" "$FIX"

mode=$(read_mode)
if cmp -s "$SRC/themes/$mode.conf" "$DROPIN"; then
  pass "colors match the saved mode ($mode)"
else
  fail "colors do not match the saved mode ($mode)" "${THEME_CMD/#$HOME/\~} apply"
fi

if pgrep -x dunst >/dev/null; then pass "dunst is running"
else info "dunst is not running (it starts on the next notification)"; fi

run_checks
