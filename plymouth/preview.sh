#!/usr/bin/env bash
# Preview the voyager theme without rebooting.
#
# Run from a text console (Ctrl+Alt+F3), NOT inside your desktop session:
# the compositor owns the display while it is in front.
#
# What happens:
#   1. ~3 s of idle starfield
#   2. prompt "one": type, try Backspace, press Enter  -> warp
#   3. prompt "two": looks like a wrong passphrase       -> red flash + shake
#      type again, press Enter                            -> warp, then quit
#
# Stuck? Ctrl+C, then:  sudo plymouth quit || sudo pkill plymouthd
# The previous default theme is restored when the script exits, even if plymouthd fails to start.

set -euo pipefail

TTY_DEV="$(tty)"
case "$TTY_DEV" in
  /dev/tty[0-9]*) ;;
  *) echo "Run this from a text console (Ctrl+Alt+F3), not from $TTY_DEV."; exit 1 ;;
esac

[[ -d /usr/share/plymouth/themes/voyager ]] || { echo "Theme not installed — run: sudo ./plymouth/install.sh"; exit 1; }

PREV_THEME="$(plymouth-set-default-theme)"
if [[ "$PREV_THEME" != voyager ]]; then
  echo "Temporarily switching default theme ($PREV_THEME -> voyager, no initramfs rebuild)…"
  sudo plymouth-set-default-theme voyager
  restore() {
    sudo plymouth-set-default-theme "$PREV_THEME" && echo "Restored default theme: $PREV_THEME"
  }
  trap restore EXIT
fi

# --tty is required: plymouthd reads the keyboard from tty1 by default
sudo sh -c "plymouthd --debug --debug-file=/tmp/plymouth.log --tty=$TTY_DEV \
  && plymouth show-splash && sleep 3 \
  && plymouth ask-for-password --prompt=one; sleep 3; \
  plymouth ask-for-password --prompt=two; sleep 4; plymouth quit"

echo "Debug log: /tmp/plymouth.log"
