#!/usr/bin/env bash
# Check that every file lib/palette lists still uses that palette colour.
# Read-only; ./doctor.sh runs this. Exit 1 if a copy has drifted.

set -uo pipefail   # no -e: every check runs, failures are reported

LIB="$(dirname "$(readlink -f "$0")")"
ROOT="$(dirname "$LIB")"
# shellcheck source=common.sh
source "$LIB/common.sh"

# patterns <#RRGGBB>: one extended regex matching the colour in any form the
# configs use: hex, rgb()/rgba() with 0-255 channels, Plymouth's 0-1 floats.
patterns() {
  local hex="${1#\#}" r g b f
  r=$((16#${hex:0:2})) g=$((16#${hex:2:2})) b=$((16#${hex:4:2}))
  read -r f < <(awk -v r="$r" -v g="$g" -v b="$b" \
    'BEGIN { printf "%.2f, *%.2f, *%.2f\n", r / 255, g / 255, b / 255 }')
  echo "#$hex|rgba?\\( *$r, *$g, *$b *[,)]|${f//./\\.}"
}

while read -r name hex files; do
  [[ -z "$name" || "$name" == \#* ]] && continue
  re="$(patterns "$hex")"
  bad=()
  for f in $files; do
    grep -qiE "$re" "$ROOT/$f" 2>/dev/null || bad+=("$f")
  done
  if (( ${#bad[@]} )); then fail "$name ($hex) is missing or changed in: ${bad[*]}" "set it back to $hex there, or update lib/palette"
  else pass "$name $hex"; fi
done < "$LIB/palette"

run_checks
