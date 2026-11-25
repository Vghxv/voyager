#!/usr/bin/env bash
# Prints the name of the active wireless connection, or nothing when offline.
# NetworkManager (nmcli) if it is running, else the SSID from iw (iwd,
# wpa_supplicant, ...), whatever the interface is called.

if command -v nmcli >/dev/null && nmcli -t general status >/dev/null 2>&1; then
  # -e no: names stay as they are; the type comes first, so a ':' in the name is fine
  nmcli -t -e no -f TYPE,NAME connection show --active \
    | awk '/^802-11-wireless:/ { sub(/^[^:]*:/, ""); print; exit }'
elif command -v iw >/dev/null; then
  iw dev | awk '$1 == "ssid" { sub(/^[ \t]*ssid /, ""); print; exit }'
else
  # polled every few seconds: say it once per login, not on every poll
  flag="${XDG_RUNTIME_DIR:-/tmp}/voyager-wifi-warned"
  [[ -e "$flag" ]] || { echo "wifi.sh: neither nmcli nor iw found — sudo pacman -S iw" >&2; touch "$flag"; }
fi
