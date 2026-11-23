#!/usr/bin/env bash
# Check the installed Plymouth theme. Read-only, no root needed; ./doctor.sh runs this.
# Docs: plymouth/plymouth.md#if-it-breaks

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DEST=/usr/share/plymouth/themes/voyager

[[ -d "$DEST" ]] || { info "not installed (sudo ./plymouth/install.sh)"; exit 0; }

check_deps_report "$SRC/deps" "plymouth"
cd "$SRC/voyager" || exit 1
check_files "theme files" "sudo ./plymouth/install.sh --apply" "$SRC/voyager" "$DEST" *

while read -r p; do
  [[ -n "$p" ]] && fail "$p" "see plymouth/plymouth.md#prerequisites"
done < <("$SRC/prereqs.sh")

if [[ "$(plymouth-set-default-theme 2>/dev/null)" == voyager ]]; then
  pass "voyager is the default theme"
  for img in /boot/initramfs-*.img; do
    [[ "$img" == *-fallback.img || ! -r "$img" ]] && continue
    if lsinitcpio "$img" 2>/dev/null | grep -q themes/voyager/voyager.script; then pass "voyager is inside $img"
    else fail "voyager is missing from $img" "sudo ./plymouth/install.sh --apply"; fi
  done
else
  info "installed but not the default theme (sudo ./plymouth/install.sh --apply)"
fi

run_checks
