#!/usr/bin/env bash
# Prints screen brightness in percent, then again on every change; nothing on
# machines without a backlight. Picks the panel's own backlight (intel_backlight,
# amdgpu_bl0, nvidia_0, ...) over the ACPI one, which often does nothing.
#
# Needs: inotifywait (inotify-tools). Errors go to `eww logs`.

source "$(dirname "$0")/lib.sh"

dev=""
for d in /sys/class/backlight/*; do
  [[ -e "$d" ]] || continue
  [[ -z "$dev" ]] && dev="$d"
  if [[ "${d##*/}" != acpi_video* ]]; then dev="$d"; break; fi
done
[[ -n "$dev" ]] || { log "no backlight in /sys/class/backlight — cell stays empty"; exec sleep infinity; }

max=$(cat "$dev/max_brightness")
get_brightness() { echo $(( ($(cat "$dev/brightness") * 100 + max / 2) / max )); }

get_brightness
need inotifywait
inotifywait -m -q -e modify "$dev/brightness" | while read -r _; do
  get_brightness
done | print_on_change
