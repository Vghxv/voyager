#!/usr/bin/env bash
# Deploy the voyager eww bar: repo -> ~/.config/eww, one way.
#
#   ./eww/install.sh
#
# Installs the whole bar (yuck, scripts, eww.scss, icons). Nothing is read back
# from ~/.config/eww and no backups are made — this repo is the source of truth.
# Files of the same name are overwritten; other files there are left alone.
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"

DEST="$HOME/.config/eww"

require_user

[[ $# -eq 0 ]] || { red "Unknown option: $1"; exit 1; }

command -v eww >/dev/null || { red "eww is not installed (it must be on PATH as 'eww')."; exit 1; }

# command -> Arch package, for everything the bar's scripts and buttons call
missing=()
for dep in hyprctl:hyprland jq:jq socat:socat inotifywait:inotify-tools pactl:libpulse \
           wpctl:wireplumber pavucontrol:pavucontrol; do
  command -v "${dep%%:*}" >/dev/null || missing+=("${dep#*:}")
done
# wifi: NetworkManager or, without it, iw
command -v nmcli >/dev/null || command -v iw >/dev/null || missing+=(iw)
if (( ${#missing[@]} )); then
  yellow "Missing packages — parts of the bar will stay empty until installed:"
  echo "  sudo pacman -S ${missing[*]}"
fi

install -d "$DEST/audio" "$DEST/scripts" "$DEST/assets"
install -m 644 "$SRC"/eww.yuck "$SRC"/var.yuck "$SRC"/utils.yuck "$SRC"/eww.scss "$DEST"/
install -m 644 "$SRC"/audio/*.yuck "$DEST/audio"/
install -m 755 "$SRC"/scripts/*.sh "$DEST/scripts"/
install -m 644 "$SRC"/icons/*.png "$DEST/assets"/
echo "Installed the bar to $DEST"

if eww ping >/dev/null 2>&1; then
  eww reload && green "Reloaded eww"
else
  echo
  echo "eww is not running. Start it now:   eww daemon && eww open bar"
  echo "Start it at login (hyprland.conf):  exec-once = eww daemon && eww open bar"
fi
