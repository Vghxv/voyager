#!/usr/bin/env bash
# Check the installed wallpaper and awww service. Read-only; ./doctor.sh runs this.
# Docs: awww/awww.md#if-it-breaks

set -uo pipefail   # no -e: every check runs, failures are reported

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"
UNIT=awww.service
FIX="./awww/install.sh"

[[ -f "$SYSTEMD_USER/$UNIT" ]] || not_installed "$FIX"

check_deps_report "$SRC/deps" "awww"
check_files "wallpaper files" "$FIX" "$SRC" "$HYPR_VOYAGER" wallpaper.webp wallpaper.png

if systemctl --user is-active -q graphical-session.target; then pass "systemd graphical session is up"
else fail "graphical-session.target is not active, so awww.service never starts" "start Hyprland through uwsm: uwsm start hyprland.desktop"; fi

if systemctl --user is-enabled -q "$UNIT" 2>/dev/null; then pass "$UNIT is enabled"
else fail "$UNIT is not enabled" "$FIX"; fi
if systemctl --user is-active -q "$UNIT"; then pass "$UNIT is running"
else fail "$UNIT is not running" "systemctl --user restart $UNIT   (why: journalctl --user -u $UNIT -n 20)"; fi

if awww query >/dev/null 2>&1; then pass "awww answers"
else fail "awww-daemon does not answer" "systemctl --user restart $UNIT"; fi

run_checks
