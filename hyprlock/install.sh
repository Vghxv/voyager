#!/usr/bin/env bash
# Install the lock screen: assets to ~/.config/hypr/voyager, config to hyprlock.conf.
#
#   ./hyprlock/install.sh            install (backs up your current hyprlock.conf)
#   ./hyprlock/install.sh --restore  put that backup back
#
# Docs: hyprlock/hyprlock.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"

HYPR="$HOME/.config/hypr"
DEST="$HYPR/voyager"
CONF="$HYPR/hyprlock.conf"

require_user

case "${1:-}" in
  --restore) restore_latest "$CONF"; exit ;;
  "") ;;
  *) red "Unknown option: $1 (use --restore or nothing)"; exit 1 ;;
esac

check_deps "$SRC/deps"

mkdir -p "$DEST"
rm -f "$DEST"/stars-*.png   # next-frame.sh loops over all of them: no leftovers from a longer run
install -m 644 "$SRC"/stars-*.png "$SRC"/probe.png "$DEST"/
warn_if_small "$SRC/stars-0.png" "src/gen-hyprlock.py --size auto"
install -m 755 "$SRC"/next-frame.sh "$DEST"/
echo "Copied assets to $DEST"

backup_file "$CONF" "$SRC/hyprlock.conf"
install -m 644 "$SRC"/hyprlock.conf "$CONF"
echo "Installed $CONF"

echo
echo "Try it:  hyprlock   (unlock with your user password)"
echo "Undo:    ./hyprlock/install.sh --restore"
