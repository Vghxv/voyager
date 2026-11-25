#!/bin/sh
# Print the starfield frame for the current second, looping over every
# stars-N.png next to this script. hyprlock's background reload_cmd runs it
# once a second and crossfades to the result.
d="$(dirname "$(readlink -f "$0")")"
n=0
for f in "$d"/stars-*.png; do [ -e "$f" ] && n=$((n + 1)); done
[ "$n" -gt 0 ] || n=1   # no frames: still print stars-0.png, hyprlock keeps the last image
printf '%s/stars-%s.png' "$d" "$(( $(date +%s) % n ))"
