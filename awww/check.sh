#!/usr/bin/env bash
# Check the installed wallpaper and awww service. Read-only; ./doctor.sh runs this.
# Docs: awww/awww.md#if-it-breaks

SRC="$(dirname "$(readlink -f "$0")")"
source "$SRC/../lib/common.sh"
DEST="$HOME/.config/hypr/voyager"
UNIT=awww.service
FIX="./awww/install.sh"

[[ -f "$HOME/.config/systemd/user/$UNIT" ]] || { info "not installed ($FIX)"; exit 0; }

check_deps_report "$SRC/deps" "awww"
check_files "wallpaper files" "$FIX" "$SRC" "$DEST" wallpaper.webp wallpaper.png

if systemctl --user is-active -q graphical-session.target; then pass "systemd graphical session is up"
else fail "graphical-session.target is not active, so awww.service never starts" "start Hyprland through uwsm: uwsm start hyprland.desktop"; fi

systemctl --user is-enabled -q "$UNIT" 2>/dev/null && pass "$UNIT is enabled" ||
  fail "$UNIT is not enabled" "$FIX"
if systemctl --user is-active -q "$UNIT"; then pass "$UNIT is running"
else fail "$UNIT is not running" "systemctl --user restart $UNIT   (why: journalctl --user -u $UNIT -n 20)"; fi

if awww query >/dev/null 2>&1; then pass "awww answers"
else fail "awww-daemon does not answer" "systemctl --user restart $UNIT"; fi

run_checks
