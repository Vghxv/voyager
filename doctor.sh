#!/usr/bin/env bash
# Check every installed voyager component and print how to fix what is broken.
#
#   ./doctor.sh            all components
#   ./doctor.sh eww dunst  only these
#
# Read-only: it changes nothing. Each component's own check.sh does the work.

cd "$(dirname "$(readlink -f "$0")")" || exit 1
source lib/common.sh

components=("$@")
(( ${#components[@]} )) || components=(plymouth hyprlock awww eww dunst)

status=0
for c in "${components[@]}"; do
  [[ -x "$c/check.sh" ]] || { red "unknown component: $c"; status=1; continue; }
  echo "$c"
  "$c/check.sh" || status=1
done

# shared light/dark switch
if [[ -d "$VOYAGER_CONF/theme.d" ]]; then
  echo "light/dark"
  mode=$(cat "$VOYAGER_CONF/theme" 2>/dev/null)
  [[ "$mode" == light || "$mode" == dark ]] && pass "saved mode: $mode" ||
    fail "saved mode is '${mode:-missing}' in $VOYAGER_CONF/theme" "~/.config/scripts/voyager-theme light"
  check_same lib/voyager-theme "$THEME_CMD" "re-run ./eww/install.sh or ./dunst/install.sh"
  for hook in "$VOYAGER_CONF"/theme.d/*; do
    [[ -x "$hook" ]] || fail "hook ${hook##*/} is not executable" "chmod +x $hook"
  done
  (( FAILED )) && status=1
fi

echo
if (( status )); then red "Something is off — run the fix lines above."
else green "All good."; fi
exit "$status"
