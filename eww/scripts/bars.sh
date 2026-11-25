#!/usr/bin/env bash
# Open the bar on every monitor: one `bar` window per monitor, id bar-<name>.
#
#   bars.sh          open a bar on each monitor, close bars of monitors that are gone
#   bars.sh toggle   close every bar if any is open, else open them
#   bars.sh watch    open them, then follow monitor hotplug (for exec-once)
#
# Needs: eww, hyprctl, jq, socat (watch). EWW=/path/to/eww for another binary.
# Docs:  eww/eww.md#start

source "$(dirname "$0")/lib.sh"

die() { log "$*"; exit 1; }
command -v "$EWW" >/dev/null || die "eww not found — install it or set EWW=/path/to/eww"
hyprctl monitors -j >/dev/null 2>&1 || die "no Hyprland session here — run it from inside Hyprland"

monitors() { hyprctl monitors -j | jq -r '.[].name'; }
open_bars() { "$EWW" active-windows | sed -n 's/^\(bar-[^:]*\):.*/\1/p'; }

sync_bars() {
  local mons id m
  mapfile -t mons < <(monitors)
  for id in $(open_bars); do
    printf '%s\n' "${mons[@]}" | grep -qxF "${id#bar-}" || "$EWW" close "$id"
  done
  # a plain `eww open bar` from before would sit on top of one of them
  if "$EWW" active-windows | grep -q '^bar:'; then "$EWW" close bar; fi
  for m in "${mons[@]}"; do
    open_bars | grep -qxF "bar-$m" ||
      "$EWW" open bar --id "bar-$m" --screen "$m" --arg "monitor=$m"
  done
}

if ! eww_running; then
  "$EWW" daemon >/dev/null 2>&1
  for _ in {1..10}; do eww_running && break; sleep 0.2; done
  eww_running || die "the eww daemon did not start — see: $EWW logs"
fi

# Set the light/dark mode before any bar opens (see `theme` in var.yuck). Same
# rule as voyager-theme: anything but "dark" is light.
mode=$(cat "$HOME/.config/voyager/theme" 2>/dev/null)
[[ "$mode" == dark ]] || mode=light
[[ "$("$EWW" get theme 2>/dev/null)" == "$mode" ]] || "$EWW" update theme="$mode"

case "${1:-}" in
  "") sync_bars ;;
  toggle)
    mapfile -t ids < <(open_bars)
    if (( ${#ids[@]} )); then "$EWW" close "${ids[@]}"; else sync_bars; fi ;;
  watch)
    # one watcher per session: two would race each other on every hotplug
    exec 9>"$XDG_RUNTIME_DIR/voyager-bars-watch.lock"
    flock -n 9 || die "another 'bars.sh watch' is already running"
    sync_bars
    [[ -S "$(hypr_socket)" ]] || die "Hyprland event socket not found ($(hypr_socket)) — hotplug will not be followed"
    hypr_events | while read -r line; do
      # give Hyprland a moment to finish setting the monitor up
      case "$line" in monitoradded'>>'*|monitorremoved'>>'*) sleep 0.5; sync_bars ;; esac
    done ;;
  *) echo "usage: $0 [toggle|watch]" >&2; exit 1 ;;
esac
