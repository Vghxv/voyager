#!/usr/bin/env bash
# Install the voyager hyprlock theme for the current user.
#
#   ./install-hyprlock.sh            install (backs up your current hyprlock.conf)
#   ./install-hyprlock.sh --restore  put the most recent backup back
#
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)/hyprlock"
HYPR="$HOME/.config/hypr"
DEST="$HYPR/voyager"
CONF="$HYPR/hyprlock.conf"

[[ $EUID -ne 0 ]] || { echo "Run as your normal user, not root."; exit 1; }

if [[ "${1:-}" == "--restore" ]]; then
  last="$(ls -1t "$CONF".bak-* 2>/dev/null | head -1 || true)"
  [[ -n "$last" ]] || { echo "No backup found ($CONF.bak-*)."; exit 1; }
  cp "$last" "$CONF"
  echo "Restored $last -> $CONF"
  exit 0
fi

mkdir -p "$DEST"
install -m 644 "$SRC"/stars-*.png "$SRC"/probe.png "$DEST"/
install -m 755 "$SRC"/next-frame.sh "$DEST"/
echo "Copied assets to $DEST"

if [[ -f "$CONF" ]]; then
  bak="$CONF.bak-$(date +%Y%m%d-%H%M%S)"
  cp "$CONF" "$bak"
  echo "Backed up existing config -> $bak"
fi
install -m 644 "$SRC"/hyprlock.conf "$CONF"
echo "Installed $CONF"

echo
echo "Test it now:  hyprlock      (unlock with your user password)"
echo "Undo:         ./install-hyprlock.sh --restore"
