#!/usr/bin/env bash
# Install the wallpaper: build voyager-sky, run awww as a systemd user service and
# show the sky (your last chosen view, or Voyager 1 in the middle the first time).
# Drag mode needs `voyager-sky listen` started with Hyprland; that line in
# hyprland.lua is yours to add (the end of this script shows it).
#
#   ./awww/install.sh            install / update (keeps the view you chose in drag mode)
#   ./awww/install.sh --restore  stop awww and put your previous unit back (or remove voyager's)
#
# Docs: awww/awww.md

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"

UNIT=awww.service
UNIT_FILE="$SYSTEMD_USER/$UNIT"
SKY_CMD="$HOME/.config/scripts/voyager-sky"

restore() {
  pkill -x voyager-sky 2>/dev/null && green "Stopped voyager-sky (listen and drag mode)" || true
  if systemctl --user disable --now "$UNIT" 2>/dev/null; then green "Stopped and disabled $UNIT"; fi
  if latest_backup "$UNIT_FILE" >/dev/null; then
    restore_latest "$UNIT_FILE"
    echo "Your own $UNIT is back but not enabled. Enable it: systemctl --user enable --now $UNIT"
  elif [[ -f "$UNIT_FILE" ]]; then
    rm -f "$UNIT_FILE"
    green "Removed $UNIT_FILE"
  fi
  systemctl --user daemon-reload
  if [[ -L "$SKY_CMD" ]]; then rm -f "$SKY_CMD"; green "Removed $SKY_CMD"; fi
  echo "The wallpaper files stay in $HYPR_VOYAGER. Remove the voyager-sky lines (listen, any toggle keybind) from hyprland.lua by hand."
}

parse_option --restore -- "$@"
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

echo "Building voyager-sky (the first build takes a few minutes)..."
cargo build --release --locked --quiet --manifest-path "$SRC/voyager-sky/Cargo.toml"

# a running listen or drag mode would keep the old binary (and drag mode would write
# over the new wallpaper); listen starts again with your next login
pkill -x voyager-sky 2>/dev/null && sleep 1 || true

require_lfs_files "$SRC"/sky/*.png "$SRC"/sky/figures/*.png
install -m 755 "$SRC/voyager-sky/target/release/voyager-sky" "$HYPR_VOYAGER/voyager-sky"
(cd "$SRC/sky" && find . -type f -exec install -D -m 644 {} "$HYPR_VOYAGER/sky/{}" \;)    # every file, as check.sh checks
mkdir -p "$(dirname "$SKY_CMD")"
ln -sfn "$HYPR_VOYAGER/voyager-sky" "$SKY_CMD"
rm -f "$HYPR_VOYAGER/wallpaper.webp"        # the old animated wallpaper
echo "Installed voyager-sky and its sky data to $HYPR_VOYAGER"

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

# render wallpaper.png and lock.png for your largest monitor from the saved view
# (Voyager 1 in the middle if there is none yet), and show it
[[ -f "$HYPR_VOYAGER/view.json" ]] || echo "No saved view yet: Voyager 1 goes in the middle."
"$HYPR_VOYAGER/voyager-sky" apply
green "Showing $HYPR_VOYAGER/wallpaper.png"

echo
echo "Drag mode: hold the left button on empty desktop. Start the listener with Hyprland, in hyprland.lua:"
echo "  hl.on(\"hyprland.start\", function() hl.exec_cmd(os.getenv(\"HOME\") .. \"/.config/scripts/voyager-sky listen\") end)"
echo "Start it now: setsid -f $SKY_CMD listen"
echo "Or from the keyboard: bind a key to \"$SKY_CMD toggle\" (Esc or toggle again sets the wallpaper)"
echo "Undo:        ./awww/install.sh --restore"
