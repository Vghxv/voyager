#!/usr/bin/env bash
# Install the lock screen: the probe to ~/.config/hypr/voyager, config to hyprlock.conf.
# Its background is the wallpaper's lock.png (awww/install.sh writes it).
#
#   ./hyprlock/install.sh            install (backs up your current hyprlock.conf)
#   ./hyprlock/install.sh --restore  put that backup back
#
# Docs: hyprlock/hyprlock.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"

CONF="$HOME/.config/hypr/hyprlock.conf"

parse_option --restore -- "$@"
require_user
if [[ "$OPT" == --restore ]]; then restore_latest "$CONF"; exit; fi

check_deps "$SRC/deps"

mkdir -p "$HYPR_VOYAGER"
rm -f "$HYPR_VOYAGER"/stars-*.png "$HYPR_VOYAGER"/next-frame.sh   # the old twinkle frames
install -m 644 "$SRC"/probe.png "$HYPR_VOYAGER"/
echo "Copied the probe to $HYPR_VOYAGER"
# the background is the wallpaper's sky: lock.png, written by voyager-sky
if [[ ! -s "$HYPR_VOYAGER/lock.png" ]]; then
  if [[ -x "$HYPR_VOYAGER/voyager-sky" ]]; then
    "$HYPR_VOYAGER/voyager-sky" apply
  else
    yellow "  ! No sky yet: the lock screen needs the wallpaper's lock.png. Install it:  ./awww/install.sh"
  fi
fi

backup_file "$CONF" "$SRC/hyprlock.conf"
install -m 644 "$SRC"/hyprlock.conf "$CONF"
echo "Installed $CONF"

echo
echo "Try it:  hyprlock   (unlock with your user password)"
echo "Undo:    ./hyprlock/install.sh --restore"
