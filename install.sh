#!/usr/bin/env bash
# voyager Plymouth theme installer
#
#   sudo ./install.sh            copy theme files only
#   sudo ./install.sh --apply    copy + set as default + rebuild initramfs
#   sudo ./install.sh --remove   delete the theme (switch default theme first!)
#
# This script never edits /etc/mkinitcpio.conf or your bootloader config.
# It only checks them and tells you what is missing.

set -euo pipefail

THEME=voyager
SRC="$(cd "$(dirname "$0")" && pwd)/theme/$THEME"
DEST="/usr/share/plymouth/themes/$THEME"

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
yellow(){ printf '\033[33m%s\033[0m\n' "$*"; }

[[ $EUID -eq 0 ]] || { red "Run with sudo."; exit 1; }
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

  if [[ $ok -eq 1 ]]; then green "  prerequisites look good"; else yellow "  see README → Prerequisites"; fi
}

case "${1:-}" in
  --remove)
    if [[ "$(plymouth-set-default-theme)" == "$THEME" ]]; then
      red "voyager is the current default. Switch first, e.g.: sudo plymouth-set-default-theme -R bgrt"
      exit 1
    fi
    rm -rf "$DEST"
    green "Removed $DEST"
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
  echo "Setting default theme and rebuilding initramfs…"
  plymouth-set-default-theme -R "$THEME"
  if lsinitcpio /boot/initramfs-linux.img 2>/dev/null | grep -q "themes/$THEME/voyager.script"; then
    green "Done — voyager is inside the initramfs. Reboot to see it."
  else
    yellow "Rebuild finished, but voyager was not found in /boot/initramfs-linux.img — check the mkinitcpio output above."
  fi
else
  echo
  echo "Next steps:"
  echo "  preview:  ./preview.sh   (run from a TTY, e.g. Ctrl+Alt+F3)"
  echo "  apply:    sudo ./install.sh --apply"
fi
