#!/usr/bin/env bash
# Install the eww bar: copy this folder to ~/.config/eww and restart eww.
#
#   ./eww/install.sh
#
# One way: the repo is the source of truth, nothing is backed up or read back.
# Docs: eww/eww.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DEST="$HOME/.config/eww"

require_user
[[ $# -eq 0 ]] || { red "Unknown option: $1 (this installer takes none)"; exit 1; }
command -v "${EWW:-eww}" >/dev/null || { red "eww is not on PATH. Install it (AUR: eww) or set EWW=/path/to/eww."; exit 1; }
check_deps "$SRC/deps"

install -d "$DEST/audio" "$DEST/scripts" "$DEST/assets/light" "$DEST/assets/dark"
install -m 644 "$SRC"/eww.yuck "$SRC"/var.yuck "$SRC"/utils.yuck "$SRC"/eww.scss "$DEST"/
install -m 644 "$SRC"/audio/*.yuck "$DEST/audio"/
rm -f "$DEST/scripts/vol.sh" "$DEST/scripts/mic.sh"   # from an earlier version: now audio.sh
install -m 755 "$SRC"/scripts/*.sh "$DEST/scripts"/
install -m 644 "$SRC"/icons/light/*.png "$DEST/assets/light"/
install -m 644 "$SRC"/icons/dark/*.png "$DEST/assets/dark"/
rm -rf "$DEST/theme.scss" "$DEST/themes"   # from an earlier version: both modes now live in eww.scss
install_theme_hook eww "$SRC/theme-hook.sh"
echo "Installed the bar to $DEST ($("$THEME_CMD") mode)"

EWW=${EWW:-eww}
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
