#!/usr/bin/env bash
# Deploy the voyager eww bar: repo -> ~/.config/eww, one way.
#
#   ./eww/install.sh
#
# Installs the whole bar (yuck, scripts, eww.scss, light and dark icons) and
# the voyager-theme switcher to ~/.config/scripts. Nothing is read back
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

install -d "$DEST/audio" "$DEST/scripts" "$DEST/assets/light" "$DEST/assets/dark"
install -m 644 "$SRC"/eww.yuck "$SRC"/var.yuck "$SRC"/utils.yuck "$SRC"/eww.scss "$DEST"/
install -m 644 "$SRC"/audio/*.yuck "$DEST/audio"/
install -m 755 "$SRC"/scripts/*.sh "$DEST/scripts"/
install -m 644 "$SRC"/icons/light/*.png "$DEST/assets/light"/
install -m 644 "$SRC"/icons/dark/*.png "$DEST/assets/dark"/
install -D -m 755 "$SRC/../lib/voyager-theme" "$HOME/.config/scripts/voyager-theme"
rm -f "$HOME/.local/bin/voyager-theme"   # from an earlier version
rm -rf "$DEST/theme.scss" "$DEST/themes"   # from an earlier version: both modes now live in eww.scss
mode="$("$HOME/.config/scripts/voyager-theme")"
echo "Installed the bar to $DEST ($mode mode; switch with: ~/.config/scripts/voyager-theme toggle)"

if eww ping >/dev/null 2>&1; then
  # eww 0.6's reload cannot reopen windows that take arguments (the bar does): restart instead
  eww kill >/dev/null 2>&1; sleep 0.5
  "$DEST/scripts/bars.sh" && green "Restarted eww"
else
  echo
  echo "eww is not running. Start it now:   $DEST/scripts/bars.sh"
  echo "Start it at login (hyprland.conf):  exec-once = $DEST/scripts/bars.sh watch"
fi
