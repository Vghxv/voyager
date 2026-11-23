#!/bin/sh
# Print the starfield frame for the current second, looping over every
# stars-N.png next to this script. hyprlock's background reload_cmd runs it
# once a second and crossfades to the result.
d="$(dirname "$(readlink -f "$0")")"
n=$(ls "$d"/stars-*.png 2>/dev/null | wc -l)
[ "$n" -gt 0 ] || n=1   # no frames: still print stars-0.png, hyprlock keeps the last image
printf '%s/stars-%s.png' "$d" "$(( $(date +%s) % n ))"
