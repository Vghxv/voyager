#!/usr/bin/env bash
# Install the Plymouth boot theme. Never edits mkinitcpio.conf or the bootloader.
#
#   sudo ./plymouth/install.sh            copy the theme, check prerequisites
#   sudo ./plymouth/install.sh --apply    also make it the default and rebuild the initramfs
#   sudo ./plymouth/install.sh --remove   delete it (switch to another theme first)
#
# Docs: plymouth/plymouth.md

set -euo pipefail

THEME=voyager
HERE="$(dirname "$(readlink -f "$0")")"
source "$HERE/../lib/common.sh"
SRC="$HERE/$THEME"
DEST="/usr/share/plymouth/themes/$THEME"
# every kernel's main initramfs (linux, linux-lts, linux-zen, ...), not the fallbacks
shopt -s nullglob
IMGS=()
for f in /boot/initramfs-*.img; do [[ "$f" == *-fallback.img ]] || IMGS+=("$f"); done
shopt -u nullglob

require_root
check_deps "$HERE/deps"

check_prereqs() {
  local problems
  problems="$("$HERE/prereqs.sh")"
  if [[ -z "$problems" ]]; then green "  prerequisites look good"
  else while read -r p; do yellow "  ! $p"; done <<<"$problems"; yellow "  see plymouth/plymouth.md#prerequisites"; fi
}

# Keep a copy of each current initramfs before rebuilding, if /boot has room.
backup_initramfs() {
  local img need free
  for img in "${IMGS[@]}"; do
    need=$(( $(stat -c %s "$img") / 1024 + 51200 ))   # image + 50 MB margin, in KB
    free=$(df --output=avail -k /boot | tail -1)
    if (( free > need )); then
      cp -p "$img" "$img.pre-voyager"
      green "  backed up $img -> $img.pre-voyager"
    else
      yellow "  ! not enough space in /boot to back up $img — continuing without a backup"
    fi
  done
}

case "${1:-}" in
  --remove)
    if [[ "$(plymouth-set-default-theme)" == "$THEME" ]]; then
      red "voyager is the current default. Switch first, e.g.: sudo plymouth-set-default-theme -R bgrt"
      exit 1
    fi
    rm -rf "$DEST"
    green "Removed $DEST"
    for img in "${IMGS[@]}"; do
      [[ -f "$img.pre-voyager" ]] && echo "Old initramfs backup still at $img.pre-voyager — delete it once you boot fine."
    done
    exit 0
    ;;
  ""|--apply) ;;
  *) red "Unknown option: $1 (use --apply, --remove or nothing)"; exit 1 ;;
esac

echo "Checking prerequisites…"
check_prereqs

echo "Installing theme to $DEST…"
install -d "$DEST"
install -m 644 "$SRC"/* "$DEST"/
green "  $(ls "$DEST" | wc -l) files installed"

if [[ "${1:-}" == "--apply" ]]; then
  prev="$(plymouth-set-default-theme)"
  echo "Previous default theme: $prev   (revert: sudo plymouth-set-default-theme -R $prev)"
  backup_initramfs
  echo "Setting default theme and rebuilding initramfs…"
  plymouth-set-default-theme -R "$THEME"
  missing=0
  for img in "${IMGS[@]}"; do
    if lsinitcpio "$img" 2>/dev/null | grep -q "themes/$THEME/voyager.script"; then
      green "  voyager is inside $img"
    else
      yellow "  ! voyager was not found in $img — check the mkinitcpio output above"; missing=1
    fi
  done
  (( ${#IMGS[@]} )) || { yellow "  ! no /boot/initramfs-*.img found — check the mkinitcpio output above"; missing=1; }
  (( missing )) || green "Done. Reboot to see it."
else
  echo
  echo "Next steps:"
  echo "  preview:  ./plymouth/preview.sh   (run from a TTY, e.g. Ctrl+Alt+F3)"
  echo "  apply:    sudo ./plymouth/install.sh --apply"
fi
