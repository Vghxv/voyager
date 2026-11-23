#!/bin/sh
# Prints the starfield frame for the current second (6-frame loop).
# hyprlock's background reload_cmd calls this every second and crossfades to the result.
d="$(dirname "$(readlink -f "$0")")"
printf '%s/stars-%s.png' "$d" "$(( $(date +%s) % 6 ))"
