#!/usr/bin/env bash
# Prints the name of the system battery (BAT0, BAT1, CMB0, ...), or nothing on
# machines without one. Skips peripheral batteries (mice, keyboards: scope=Device).

for p in /sys/class/power_supply/*; do
  [[ "$(cat "$p/type" 2>/dev/null)" == Battery ]] || continue
  [[ "$(cat "$p/scope" 2>/dev/null)" == Device ]] && continue
  basename "$p"
  exit 0
done
