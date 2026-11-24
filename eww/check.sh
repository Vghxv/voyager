#!/usr/bin/env bash
# Check the installed eww bar. Read-only; ./doctor.sh runs this.
# Docs: eww/eww.md#if-it-breaks

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DEST="$HOME/.config/eww"
EWW=${EWW:-eww}
FIX="./eww/install.sh"

[[ -f "$DEST/eww.yuck" ]] || { info "not installed ($FIX)"; exit 0; }

command -v "$EWW" >/dev/null && pass "eww is on PATH" || fail "eww not found" "install eww (AUR) or set EWW=/path/to/eww"
check_deps_report "$SRC/deps" "bar scripts"

cd "$SRC" || exit 1
files=(*.yuck eww.scss scripts/*.sh)
check_files "config and scripts" "$FIX" "$SRC" "$DEST" "${files[@]}"
cd "$SRC/icons" || exit 1
check_files "icons" "$FIX" "$SRC/icons" "$DEST/assets" light/*.png dark/*.png
check_same "$SRC/theme-hook.sh" "$VOYAGER_CONF/theme.d/eww" "$FIX"

if ! "$EWW" ping >/dev/null 2>&1; then
  fail "eww daemon is not running" "$DEST/scripts/bars.sh   (errors: $EWW logs)"
elif ! command -v hyprctl >/dev/null || ! hyprctl monitors -j >/dev/null 2>&1; then
  info "not inside Hyprland: skipped the per-monitor bar check"
else
  open=$("$EWW" active-windows 2>/dev/null)
  missing=()
  for m in $(hyprctl monitors -j | jq -r '.[].name'); do
    grep -q "^bar-$m:" <<<"$open" || missing+=("$m")
  done
  if (( ${#missing[@]} )); then fail "no bar on: ${missing[*]}" "$DEST/scripts/bars.sh   (errors: $EWW logs)"
  else pass "a bar is open on every monitor"; fi
fi

run_checks
