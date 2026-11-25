#!/usr/bin/env bash
# Install the wallpaper and run it with awww as a systemd user service.
#
#   ./awww/install.sh            animated
#   ./awww/install.sh --static   still image
#   ./awww/install.sh --restore  stop awww and put your previous unit back (or remove voyager's)
#
# Docs: awww/awww.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"

UNIT=awww.service
UNIT_FILE="$SYSTEMD_USER/$UNIT"

restore() {
  if systemctl --user disable --now "$UNIT" 2>/dev/null; then green "Stopped and disabled $UNIT"; fi
  if latest_backup "$UNIT_FILE" >/dev/null; then
    restore_latest "$UNIT_FILE"
    echo "Your own $UNIT is back but not enabled. Enable it: systemctl --user enable --now $UNIT"
  elif [[ -f "$UNIT_FILE" ]]; then
    rm -f "$UNIT_FILE"
    green "Removed $UNIT_FILE"
  fi
  systemctl --user daemon-reload
  echo "The wallpaper files stay in $HYPR_VOYAGER."
}

parse_option --static --restore -- "$@"
require_user
if [[ "$OPT" == --restore ]]; then restore; exit; fi

check_deps "$SRC/deps"

# awww.service starts with graphical-session.target and needs WAYLAND_DISPLAY in
# the systemd user environment. A session manager (e.g. uwsm) provides both;
# plain `Hyprland` from a TTY does not, and the unit would silently never start.
if ! systemctl --user is-active -q graphical-session.target \
   || ! systemctl --user show-environment | grep -q '^WAYLAND_DISPLAY='; then
  red "No systemd graphical session (graphical-session.target inactive or WAYLAND_DISPLAY not exported)."
  echo "Start Hyprland through uwsm (uwsm start hyprland.desktop), then re-run this script."
  exit 1
fi

mkdir -p "$HYPR_VOYAGER"
install -m 644 "$SRC/wallpaper.webp" "$SRC/wallpaper.png" "$HYPR_VOYAGER/"
echo "Copied wallpaper to $HYPR_VOYAGER"
warn_if_small "$SRC/wallpaper.png" "src/gen-wallpaper.py --size auto"

# point ExecStart at wherever awww-daemon is installed (/usr/bin, ~/.local/bin, ~/.cargo/bin, ...)
unit="$(mktemp)"; trap 'rm -f "$unit"' EXIT
daemon="$(command -v awww-daemon)"
daemon="${daemon//\\/\\\\}"; daemon="${daemon//#/\\#}"; daemon="${daemon//&/\\&}"
sed "s#^ExecStart=/usr/bin/awww-daemon#ExecStart=$daemon#" "$SRC/$UNIT" > "$unit"
backup_file "$UNIT_FILE" "$unit"
install -D -m 644 "$unit" "$UNIT_FILE"
systemctl --user daemon-reload
systemctl --user enable --now "$UNIT"

for _ in {1..20}; do awww query >/dev/null 2>&1 && break; sleep 0.25; done
awww query >/dev/null 2>&1 || { red "awww-daemon did not come up — see: journalctl --user -u $UNIT"; exit 1; }

img="$HYPR_VOYAGER/wallpaper.webp"
[[ "$OPT" == --static ]] && img="$HYPR_VOYAGER/wallpaper.png"
# awww caches decoded animation frames by path + size; after a re-render the path is the
# same, so a stale cache would play the old frames over the new first frame.
awww clear-cache
awww img "$img" --transition-type fade --transition-duration 1.5
green "Showing $img"
echo "Undo:  ./awww/install.sh --restore"
