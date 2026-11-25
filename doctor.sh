#!/usr/bin/env bash
# Check every installed voyager component and print how to fix what is broken.
#
#   ./doctor.sh            all components
#   ./doctor.sh eww dunst  only these
#
# Read-only: it changes nothing. Each component's own check.sh does the work.

set -uo pipefail   # no -e: every check runs, failures are reported

cd "$(dirname "$(readlink -f "$0")")" || exit 1
# shellcheck source=lib/common.sh
source lib/common.sh

# every folder with a check.sh, boot first and the notifications last
ORDER=(plymouth hyprlock awww eww dunst)
known=()
for c in "${ORDER[@]}" */; do
  c="${c%/}"
  [[ -x "$c/check.sh" && " ${known[*]} " != *" $c "* ]] && known+=("$c")
done

components=("$@")
(( ${#components[@]} )) || components=("${known[@]}")

status=0
for c in "${components[@]}"; do
  if [[ " ${known[*]} " != *" $c "* ]]; then
    red "unknown component: $c (known: ${known[*]})"; status=1; continue
  fi
  echo "$c"
  "$c/check.sh" || status=1
done

# shared light/dark switch
if [[ -d "$THEME_HOOKS" ]]; then
  echo "light/dark"
  mode=$(cat "$VOYAGER_CONF/theme" 2>/dev/null)
  if [[ "$mode" == light || "$mode" == dark ]]; then pass "saved mode: $mode"
  else fail "saved mode is '${mode:-missing}' in $VOYAGER_CONF/theme" "${THEME_CMD/#$HOME/\~} light"; fi
  check_same lib/voyager-theme "$THEME_CMD" "re-run ./eww/install.sh or ./dunst/install.sh"
  for hook in "$THEME_HOOKS"/*; do
    [[ -e "$hook" ]] || continue
    [[ -x "$hook" ]] || fail "hook ${hook##*/} is not executable" "chmod +x $hook"
  done
  (( FAILED )) && status=1
fi

echo
if (( status )); then red "Something is off — run the fix lines above."
else green "All good."; fi
exit "$status"
