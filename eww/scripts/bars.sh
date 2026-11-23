#!/usr/bin/env bash
# Open the bar on every monitor: one `bar` window per monitor, id bar-<name>.
#
#   bars.sh          open a bar on each monitor, close bars of monitors that are gone
#   bars.sh toggle   close every bar if any is open, else open them
#   bars.sh watch    open them, then follow monitor hotplug (for exec-once)
#
# Uses `eww` from PATH; set EWW=/path/to/eww to use another binary.

EWW=${EWW:-eww}

monitors() { hyprctl monitors -j | jq -r '.[].name'; }
open_bars() { $EWW active-windows | sed -n 's/^\(bar-[^:]*\):.*/\1/p'; }

sync_bars() {
  local mons id m
  mons=$(monitors)
  for id in $(open_bars); do
    grep -qxF "${id#bar-}" <<<"$mons" || $EWW close "$id"
  done
  # a plain `eww open bar` from before would sit on top of one of them
  $EWW active-windows | grep -q '^bar:' && $EWW close bar
  for m in $mons; do
    open_bars | grep -qxF "bar-$m" ||
      $EWW open bar --id "bar-$m" --screen "$m" --arg "monitor=$m"
  done
}

$EWW ping >/dev/null 2>&1 || { $EWW daemon >/dev/null 2>&1 && sleep 0.5; }

case "${1:-}" in
  "")     sync_bars ;;
  toggle) ids=$(open_bars); [[ -n "$ids" ]] && $EWW close $ids || sync_bars ;;
  watch)
    sync_bars
    socat -u UNIX-CONNECT:"$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock" - |
      while read -r line; do
        case "$line" in monitoradded*|monitorremoved*) sleep 0.5; sync_bars ;; esac
      done ;;
  *) echo "usage: $0 [toggle|watch]" >&2; exit 1 ;;
esac
