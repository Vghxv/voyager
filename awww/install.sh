#!/usr/bin/env bash
# Install the voyager wallpaper with awww (the renamed swww).
#
#   ./awww/install.sh            animated: looping WebP (twinkling stars, ring pulse)
#   ./awww/install.sh --static   still PNG instead (no CPU cost)
#   ./awww/install.sh --restore  stop awww, give the wallpaper back to hyprpaper
#
# awww replaces hyprpaper: both draw on the background layer, so this disables
# hyprpaper.service (and remembers it did, for --restore).
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
source "$SRC/../lib/common.sh"

HYPR="$HOME/.config/hypr"
DEST="$HYPR/voyager"
UNIT=awww.service
UNIT_DIR="$HOME/.config/systemd/user"
MARK="$DEST/.hyprpaper-was-enabled"
PAPER_CONF="$HYPR/hyprpaper.conf"

require_user

restore() {
  systemctl --user disable --now "$UNIT" 2>/dev/null && green "Stopped and disabled $UNIT" || true
  # an earlier voyager install may have pointed hyprpaper.conf at voyager — undo that too
  if [[ -f "$PAPER_CONF" ]] && grep -q 'hypr/voyager/' "$PAPER_CONF" && ls "$PAPER_CONF".bak-* >/dev/null 2>&1; then
    restore_latest "$PAPER_CONF"
  fi
  if [[ -f "$MARK" ]]; then
    systemctl --user enable --now hyprpaper && green "Re-enabled hyprpaper"
    rm -f "$MARK"
  else
    echo "hyprpaper was not enabled before voyager — left as is"
  fi
}

case "${1:-}" in
  --restore) restore; exit ;;
  ""|--static) ;;
  *) red "Unknown option: $1"; exit 1 ;;
esac

command -v awww-daemon >/dev/null || { red "awww is not installed (sudo pacman -S awww)."; exit 1; }

mkdir -p "$DEST"
install -m 644 "$SRC/wallpaper.webp" "$SRC/wallpaper.png" "$DEST/"
echo "Copied wallpaper to $DEST"

# hand the background over from hyprpaper to awww
if systemctl --user is-enabled -q hyprpaper 2>/dev/null; then
  touch "$MARK"
  systemctl --user disable --now hyprpaper
  green "Disabled hyprpaper (./awww/install.sh --restore turns it back on)"
elif pgrep -x hyprpaper >/dev/null; then
  pkill -x hyprpaper
  yellow "Stopped a running hyprpaper — remove it from your autostart if it is launched there"
fi

backup_file "$UNIT_DIR/$UNIT" "$SRC/$UNIT"
install -D -m 644 "$SRC/$UNIT" "$UNIT_DIR/$UNIT"
systemctl --user daemon-reload
systemctl --user enable --now "$UNIT"

for _ in $(seq 20); do awww query >/dev/null 2>&1 && break; sleep 0.25; done
awww query >/dev/null 2>&1 || { red "awww-daemon did not come up — see: journalctl --user -u $UNIT"; exit 1; }

img="$DEST/wallpaper.webp"; [[ "${1:-}" == "--static" ]] && img="$DEST/wallpaper.png"
awww img "$img" --transition-type fade --transition-duration 1.5
green "Showing $img"

echo
echo "Static:  ./awww/install.sh --static      Undo:  ./awww/install.sh --restore"
