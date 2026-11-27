#!/usr/bin/env bash
# Install the lock screen: the probe to ~/.config/hypr/voyager, config to hyprlock.conf.
# The background is ~/.config/hypr/voyager/lock.png if there is one, else plain navy.
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

require_lfs_files "$SRC"/probe.png
mkdir -p "$HYPR_VOYAGER"
rm -f "$HYPR_VOYAGER"/stars-*.png "$HYPR_VOYAGER"/next-frame.sh   # the old twinkle frames
install -m 644 "$SRC"/probe.png "$HYPR_VOYAGER"/
echo "Copied the probe to $HYPR_VOYAGER"
[[ -s "$HYPR_VOYAGER/lock.png" ]] || yellow "  ! No $HYPR_VOYAGER/lock.png: the background is plain navy"

backup_file "$CONF" "$SRC/hyprlock.conf"
install -m 644 "$SRC"/hyprlock.conf "$CONF"
echo "Installed $CONF"

echo
echo "Try it:  hyprlock   (unlock with your user password)"
echo "Undo:    ./hyprlock/install.sh --restore"
