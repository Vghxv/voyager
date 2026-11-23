#!/usr/bin/env bash
# voyager Plymouth theme installer
#
#   sudo ./plymouth/install.sh            copy theme files only
#   sudo ./plymouth/install.sh --apply    copy + set as default + rebuild initramfs
#   sudo ./plymouth/install.sh --remove   delete the theme (switch default theme first!)
#
# This script never edits /etc/mkinitcpio.conf or your bootloader config.
# It only checks them and tells you what is missing.

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
command -v plymouth-set-default-theme >/dev/null || { red "plymouth is not installed (pacman -S plymouth)."; exit 1; }

check_prereqs() {
  local ok=1 hooks
  hooks="$(grep -E '^HOOKS=' /etc/mkinitcpio.conf || true)"

  for h in systemd plymouth sd-encrypt; do
    if ! grep -qw "$h" <<<"$hooks"; then
      yellow "  ! HOOKS is missing '$h'"; ok=0
    fi
  done
  if grep -qw plymouth <<<"$hooks" && grep -qw sd-encrypt <<<"$hooks"; then
    local order
    order="$(grep -oE 'systemd|plymouth|sd-encrypt' <<<"$hooks" | tr '\n' ' ')"
    [[ "$order" == "systemd plymouth sd-encrypt " ]] || { yellow "  ! hook order should be: systemd … plymouth … sd-encrypt (got: $order)"; ok=0; }
  fi
  grep -qw splash /proc/cmdline || { yellow "  ! 'splash' is not on the running kernel cmdline"; ok=0; }

  if [[ $ok -eq 1 ]]; then green "  prerequisites look good"; else yellow "  see plymouth/plymouth.md → Prerequisites"; fi
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
  *) red "Unknown option: $1"; exit 1 ;;
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
