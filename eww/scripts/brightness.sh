#!/usr/bin/env bash
# Prints screen brightness in percent, then again on every change.
# Uses the first backlight device; prints nothing on machines without one.

dev=$(ls -d /sys/class/backlight/* 2>/dev/null | head -1)
[[ -n "$dev" ]] || exec sleep infinity

max=$(cat "$dev/max_brightness")

get_brightness() {
  echo $(( $(cat "$dev/brightness") * 100 / max ))
}

get_brightness

inotifywait -m -e modify "$dev/brightness" 2>/dev/null | while read -r _; do
  get_brightness
done
