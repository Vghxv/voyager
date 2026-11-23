#!/usr/bin/env bash
# Install the voyager hyprlock theme for the current user.
#
#   ./hyprlock/install.sh            install (backs up your current hyprlock.conf)
#   ./hyprlock/install.sh --restore  put the most recent backup back
#
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
source "$SRC/../lib/common.sh"

HYPR="$HOME/.config/hypr"
DEST="$HYPR/voyager"
CONF="$HYPR/hyprlock.conf"

require_user

case "${1:-}" in
  --restore) restore_latest "$CONF"; exit ;;
  "") ;;
  *) red "Unknown option: $1"; exit 1 ;;
esac

mkdir -p "$DEST"
install -m 644 "$SRC"/stars-*.png "$SRC"/probe.png "$DEST"/
install -m 755 "$SRC"/next-frame.sh "$DEST"/
echo "Copied assets to $DEST"

backup_file "$CONF" "$SRC/hyprlock.conf"
install -m 644 "$SRC"/hyprlock.conf "$CONF"
echo "Installed $CONF"

echo
echo "Test it now:  hyprlock      (unlock with your user password)"
echo "Undo:         ./hyprlock/install.sh --restore"
