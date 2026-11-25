#!/usr/bin/env bash
# Install the lock screen: assets to ~/.config/hypr/voyager, config to hyprlock.conf.
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
rm -f "$HYPR_VOYAGER"/stars-*.png   # next-frame.sh loops over all of them: no leftovers from a longer run
install -m 644 "$SRC"/stars-*.png "$SRC"/probe.png "$HYPR_VOYAGER"/
warn_if_small "$SRC/stars-0.png" "src/gen-hyprlock.py --size auto"
install -m 755 "$SRC"/next-frame.sh "$HYPR_VOYAGER"/
echo "Copied assets to $HYPR_VOYAGER"

backup_file "$CONF" "$SRC/hyprlock.conf"
install -m 644 "$SRC"/hyprlock.conf "$CONF"
echo "Installed $CONF"

echo
echo "Try it:  hyprlock   (unlock with your user password)"
echo "Undo:    ./hyprlock/install.sh --restore"
