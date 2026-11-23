#!/usr/bin/env bash
# Switch to workspace $1. Called by the bar's workspace buttons.
#
# Hyprland with a Lua config (hyprland.lua) parses `hyprctl dispatch` as Lua,
# so the old `dispatch workspace N` fails there. Try the Lua form first and
# fall back to the classic one for hyprland.conf setups.

ws="$1"
[[ "$ws" =~ ^-?[0-9]+$ ]] || { echo "usage: $0 <workspace id>" >&2; exit 1; }

[[ "$(hyprctl dispatch "hl.dsp.focus({ workspace = $ws })" 2>&1)" == ok ]] ||
  hyprctl dispatch workspace "$ws"
