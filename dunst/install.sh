#!/usr/bin/env bash
# Install the dunst notification theme.
#
#   ./dunst/install.sh            install (backs up your current dunstrc)
#   ./dunst/install.sh --restore  put that backup back, remove voyager's colors
#
# Docs: dunst/dunst.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DIR="$HOME/.config/dunst"
CONF="$DIR/dunstrc"
DROPIN="$DIR/dunstrc.d/90-voyager-theme.conf"
HOOK="$VOYAGER_CONF/theme.d/dunst"

require_user

reload() {
  # dunst is D-Bus activated: if it is not running, the next notification starts it
  # with the new config. Otherwise reload in place, or stop it if that fails.
  pgrep -x dunst >/dev/null || return 0
  if dunstctl reload >/dev/null 2>&1; then
    green "Reloaded dunst"
  else
    pkill -x dunst || true
    green "Stopped dunst — it restarts with the new config on the next notification"
  fi
}

case "${1:-}" in
  --restore) rm -f "$DROPIN" "$HOOK"; restore_latest "$CONF" || true; reload; exit ;;
  "") ;;
  *) red "Unknown option: $1 (use --restore or nothing)"; exit 1 ;;
esac

check_deps "$SRC/deps"

backup_file "$CONF" "$SRC/dunstrc"
install -D -m 644 "$SRC/dunstrc" "$CONF"
install -d "$DIR/themes"
install -m 644 "$SRC"/themes/*.conf "$DIR/themes"/
install_theme_hook dunst "$SRC/theme-hook.sh"
echo "Installed $CONF ($("$THEME_CMD") mode)"
reload

echo
echo "Try it:  notify-send -u critical voyager 'signal lost'"
echo "Undo:    ./dunst/install.sh --restore"
