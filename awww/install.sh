#!/usr/bin/env bash
# Install the voyager wallpaper with awww (the renamed swww).
#
#   ./awww/install.sh            animated: looping WebP (twinkling stars, ring pulse)
#   ./awww/install.sh --static   still PNG instead (no CPU cost)
#   ./awww/install.sh --restore  stop and disable awww
#
# awww is the only wallpaper daemon: hyprpaper is not used or touched.
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"

HYPR="$HOME/.config/hypr"
DEST="$HYPR/voyager"
UNIT=awww.service
UNIT_DIR="$HOME/.config/systemd/user"

require_user

restore() {
  systemctl --user disable --now "$UNIT" 2>/dev/null && green "Stopped and disabled $UNIT" || true
}

case "${1:-}" in
  --restore) restore; exit ;;
  ""|--static) ;;
  *) red "Unknown option: $1"; exit 1 ;;
esac

command -v awww-daemon >/dev/null || { red "awww is not installed (sudo pacman -S awww)."; exit 1; }

# awww.service starts with graphical-session.target and needs WAYLAND_DISPLAY in
# the systemd user environment. A session manager (e.g. uwsm) provides both;
# plain `Hyprland` from a TTY does not, and the unit would silently never start.
if ! systemctl --user is-active -q graphical-session.target \
   || ! systemctl --user show-environment | grep -q '^WAYLAND_DISPLAY='; then
  red "No systemd graphical session (graphical-session.target inactive or WAYLAND_DISPLAY not exported)."
  echo "Start Hyprland through uwsm (uwsm start hyprland.desktop), then re-run this script."
  exit 1
fi

mkdir -p "$DEST"
install -m 644 "$SRC/wallpaper.webp" "$SRC/wallpaper.png" "$DEST/"
echo "Copied wallpaper to $DEST"
warn_if_small "$SRC/wallpaper.png" "src/gen-wallpaper.py --size auto"

# point ExecStart at wherever awww-daemon is installed (/usr/bin, ~/.local/bin, ~/.cargo/bin, ...)
unit="$(mktemp)"; trap 'rm -f "$unit"' EXIT
sed "s|^ExecStart=/usr/bin/awww-daemon|ExecStart=$(command -v awww-daemon)|" "$SRC/$UNIT" > "$unit"
backup_file "$UNIT_DIR/$UNIT" "$unit"
install -D -m 644 "$unit" "$UNIT_DIR/$UNIT"
systemctl --user daemon-reload
systemctl --user enable --now "$UNIT"

for _ in $(seq 20); do awww query >/dev/null 2>&1 && break; sleep 0.25; done
awww query >/dev/null 2>&1 || { red "awww-daemon did not come up — see: journalctl --user -u $UNIT"; exit 1; }

img="$DEST/wallpaper.webp"; [[ "${1:-}" == "--static" ]] && img="$DEST/wallpaper.png"
# awww caches decoded animation frames by path + size; after a re-render the path is the
# same, so a stale cache would play the old frames over the new first frame.
awww clear-cache
awww img "$img" --transition-type fade --transition-duration 1.5
green "Showing $img"

echo
echo "Static:  ./awww/install.sh --static      Undo:  ./awww/install.sh --restore"
