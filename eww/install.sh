#!/usr/bin/env bash
# Deploy the voyager eww theme: repo -> ~/.config/eww, one way.
#
#   ./eww/install.sh
#
# Copies eww/eww.scss and eww/icons/*.png over ~/.config/eww/eww.scss and
# ~/.config/eww/assets/. Nothing is read back from ~/.config/eww and no backups
# are made — this repo is the source of truth. Your .yuck files are not touched.
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
source "$SRC/../lib/common.sh"

DEST="$HOME/.config/eww"

require_user

[[ $# -eq 0 ]] || { red "Unknown option: $1"; exit 1; }

install -D -m 644 "$SRC/eww.scss" "$DEST/eww.scss"
echo "Installed $DEST/eww.scss"

install -d "$DEST/assets"
install -m 644 "$SRC"/icons/*.png "$DEST/assets/"
echo "Copied icons to $DEST/assets"

if command -v eww >/dev/null && eww ping >/dev/null 2>&1; then
  eww reload && green "Reloaded eww"
else
  echo "eww is not running — the theme applies on next start"
fi
