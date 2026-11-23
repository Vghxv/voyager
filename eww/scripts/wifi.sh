#!/usr/bin/env bash
# Prints the name of the active wireless connection, or nothing when offline.
# NetworkManager (nmcli) if it is running, else the SSID from iw (iwd,
# wpa_supplicant, ...), whatever the interface is called.

if command -v nmcli >/dev/null && nmcli -t general status >/dev/null 2>&1; then
  nmcli -t -f NAME,TYPE connection show --active | awk -F: '$2 == "802-11-wireless" { print $1; exit }'
elif command -v iw >/dev/null; then
  iw dev | awk '$1 == "ssid" { sub(/^[ \t]*ssid /, ""); print; exit }'
fi
