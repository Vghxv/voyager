#!/usr/bin/env bash
# Check the installed Plymouth theme. Read-only, no root needed; ./doctor.sh runs this.
# Docs: plymouth/plymouth.md#if-it-breaks

set -uo pipefail   # no -e: every check runs, failures are reported

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"
DEST="$PLYMOUTH_THEME_DIR"
FIX="sudo ./plymouth/install.sh --apply"

[[ -d "$DEST" ]] || not_installed "sudo ./plymouth/install.sh"

check_deps_report "$SRC/deps" "plymouth"
cd "$SRC/voyager" || exit 1
check_files "theme files" "$FIX" "$SRC/voyager" "$DEST" *
extra=()
for f in "$DEST"/*; do [[ -e "$SRC/voyager/${f##*/}" ]] || extra+=("${f##*/}"); done
(( ${#extra[@]} )) && fail "leftover files in $DEST: ${extra[*]}" "$FIX"

while read -r p; do
  [[ -n "$p" ]] && fail "$p" "see plymouth/plymouth.md#prerequisites"
done < <("$SRC/prereqs.sh")

if [[ "$(plymouth-set-default-theme 2>/dev/null)" == voyager ]]; then
  pass "voyager is the default theme"
  while read -r img; do
    [[ -r "$img" ]] || continue
    if has_voyager_plymouth "$img"; then pass "voyager is inside $img"
    else fail "voyager is missing from $img" "$FIX"; fi
  done < <(initramfs_images)
else
  info "installed but not the default theme (sudo ./plymouth/install.sh --apply)"
fi

run_checks
