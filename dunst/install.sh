#!/usr/bin/env bash
# Install the voyager dunst theme for the current user.
#
#   ./dunst/install.sh            install (backs up your current dunstrc)
#   ./dunst/install.sh --restore  put the most recent backup back
#
# Run as your normal user — NOT with sudo.

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"

CONF="$HOME/.config/dunst/dunstrc"

require_user

reload() {
  # dunst is D-Bus activated: if it is not running, the next notification starts it
  # with the new config. Otherwise reload in place, or restart if that fails.
  pgrep -x dunst >/dev/null || return 0
  if dunstctl reload >/dev/null 2>&1; then
    green "Reloaded dunst"
  else
    pkill -x dunst || true
    green "Stopped dunst — it restarts with the new config on the next notification"
  fi
}

case "${1:-}" in
  --restore) restore_latest "$CONF"; reload; exit ;;
  "") ;;
  *) red "Unknown option: $1"; exit 1 ;;
esac

command -v dunst >/dev/null || { red "dunst is not installed (sudo pacman -S dunst)."; exit 1; }
command -v notify-send >/dev/null || yellow "notify-send is missing (sudo pacman -S libnotify) — needed for the test below."

backup_file "$CONF" "$SRC/dunstrc"
install -D -m 644 "$SRC/dunstrc" "$CONF"
echo "Installed $CONF"
reload

echo
echo "Test it now:"
echo "  notify-send -u low      voyager 'low: telemetry nominal'"
echo "  notify-send -u normal   voyager 'normal: signal acquired'"
echo "  notify-send -u critical voyager 'critical: signal lost'"
echo "  notify-send -h int:value:60 voyager 'downlink 60%'"
echo "Undo:  ./dunst/install.sh --restore"
