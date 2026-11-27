#!/usr/bin/env bash
# Install the eww bar: copy this folder to ~/.config/eww and restart eww.
#
#   ./eww/install.sh
#
# One way: the repo is the source of truth, nothing is backed up or read back.
# Set EWW=/path/to/eww to use an eww that is not on PATH.
# Docs: eww/eww.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"

DEST="$HOME/.config/eww"
EWW="${EWW:-eww}"

parse_option -- "$@"
require_user
command -v "$EWW" >/dev/null || { red "eww is not on PATH. Install it (AUR: eww) or set EWW=/path/to/eww."; exit 1; }
check_deps "$SRC/deps"

require_lfs_files "$SRC"/icons/*/*.png
install -d "$DEST/scripts" "$DEST/assets/light" "$DEST/assets/dark"
install -m 644 "$SRC"/*.yuck "$SRC"/eww.scss "$DEST"/
install -m 755 "$SRC"/scripts/*.sh "$DEST/scripts"/
install -m 644 "$SRC"/icons/light/*.png "$DEST/assets/light"/
install -m 644 "$SRC"/icons/dark/*.png "$DEST/assets/dark"/
install_theme_hook eww "$SRC/theme-hook.sh"
echo "Installed the bar to $DEST ($(read_mode) mode)"

if "$EWW" ping >/dev/null 2>&1; then
  # eww 0.6's reload cannot reopen windows that take arguments (the bar does): restart instead
  "$EWW" kill >/dev/null 2>&1 || true
  sleep 0.5
  if "$DEST/scripts/bars.sh"; then
    green "Restarted eww"
  else
    red "eww did not come back up. See why:  $EWW logs"
    exit 1
  fi
else
  echo "eww is not running. Start it:  $DEST/scripts/bars.sh   (at login: see eww/eww.md#start)"
fi
