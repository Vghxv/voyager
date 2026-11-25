#!/usr/bin/env bash
# Preview the Plymouth theme without rebooting. Run it from a text console
# (Ctrl+Alt+F3): the compositor owns the display while it is in front.
#
#   ./plymouth/preview.sh
#
# Switches the default theme for the preview only (no initramfs rebuild) and
# switches back on exit, even on failure.
# Stuck? Ctrl+C, then:  sudo plymouth quit || sudo pkill plymouthd
# Docs: plymouth/plymouth.md#preview

set -euo pipefail

SRC="$(dirname "$(readlink -f "$0")")"
# shellcheck source=../lib/common.sh
source "$SRC/../lib/common.sh"

parse_option -- "$@"

TTY_DEV="$(tty)"
case "$TTY_DEV" in
  /dev/tty[0-9]*) ;;
  *) red "Run this from a text console (Ctrl+Alt+F3), not from $TTY_DEV."; exit 1 ;;
esac

[[ -d "$PLYMOUTH_THEME_DIR" ]] || { red "Theme not installed — run: sudo ./plymouth/install.sh"; exit 1; }

PREV_THEME="$(plymouth-set-default-theme)"
if [[ "$PREV_THEME" != voyager ]]; then
  echo "Temporarily switching default theme ($PREV_THEME -> voyager, no initramfs rebuild)…"
  sudo plymouth-set-default-theme voyager
  restore() {
    sudo plymouth-set-default-theme "$PREV_THEME" && green "Restored default theme: $PREV_THEME"
  }
  trap restore EXIT
fi

LOG="$(mktemp --tmpdir voyager-plymouth.XXXXXX.log)"
# --tty is required: plymouthd reads the keyboard from tty1 by default.
# Each step runs only if plymouthd came up; quit always runs.
sudo sh -c "plymouthd --debug --debug-file='$LOG' --tty='$TTY_DEV' || exit 1
  plymouth show-splash && sleep 3 \
    && plymouth ask-for-password --prompt=one; sleep 3
  plymouth ask-for-password --prompt=two; sleep 4
  plymouth quit"
sudo chown "$(id -u):$(id -g)" "$LOG" 2>/dev/null || true

echo "Debug log: $LOG"
