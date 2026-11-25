#!/usr/bin/env bash
# Install the Plymouth boot theme. Never edits mkinitcpio.conf or the bootloader.
#
#   sudo ./plymouth/install.sh            copy the theme, check prerequisites
#   sudo ./plymouth/install.sh --apply    also make it the default and rebuild the initramfs
#   sudo ./plymouth/install.sh --remove   delete it (switch to another theme first)
#
# Docs: plymouth/plymouth.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"

THEME=voyager
THEME_SRC="$SRC/$THEME"
DEST="$PLYMOUTH_THEME_DIR"

parse_option --apply --remove -- "$@"
require_root
mapfile -t IMGS < <(initramfs_images)

check_prereqs() {
  local problems p
  problems="$("$SRC/prereqs.sh")"
  if [[ -z "$problems" ]]; then
    green "  prerequisites look good"
  else
    while read -r p; do yellow "  ! $p"; done <<<"$problems"
    yellow "  see plymouth/plymouth.md#prerequisites"
  fi
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

if [[ "$OPT" == --remove ]]; then
  if [[ "$(plymouth-set-default-theme)" == "$THEME" ]]; then
    red "voyager is the current default. Switch first, e.g.: sudo plymouth-set-default-theme -R bgrt"
    exit 1
  fi
  rm -rf "$DEST"
  green "Removed $DEST"
  for img in "${IMGS[@]}"; do
    if [[ -f "$img.pre-voyager" ]]; then
      echo "Old initramfs backup still at $img.pre-voyager — delete it once you boot fine."
    fi
  done
  exit 0
fi

check_deps "$SRC/deps"

echo "Checking prerequisites…"
check_prereqs

echo "Installing theme to $DEST…"
rm -rf "$DEST"   # no leftovers from an earlier version end up in the initramfs
install -d "$DEST"
install -m 644 "$THEME_SRC"/* "$DEST"/
files=("$DEST"/*)
green "  ${#files[@]} files installed"

if [[ "$OPT" == --apply ]]; then
  prev="$(plymouth-set-default-theme)"
  echo "Previous default theme: $prev   (revert: sudo plymouth-set-default-theme -R $prev)"
  backup_initramfs
  echo "Setting default theme and rebuilding initramfs…"
  plymouth-set-default-theme -R "$THEME"
  missing=0
  for img in "${IMGS[@]}"; do
    if has_voyager_plymouth "$img"; then
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
