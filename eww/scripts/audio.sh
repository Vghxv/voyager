#!/usr/bin/env bash
# Print the default speaker or mic volume (0-100, or "Muted"), then again on every
# change, including a switch of the default device.
#
#   audio.sh sink     speaker
#   audio.sh source   microphone
#
# Needs: pactl (libpulse). Errors go to `eww logs`.

source "$(dirname "$0")/lib.sh"
export LC_ALL=C   # pactl's messages are translated; parse the English ones

kind="${1:-}"
[[ "$kind" == sink || "$kind" == source ]] || { echo "usage: $0 sink|source" >&2; exit 1; }
need pactl
dev="@DEFAULT_${kind^^}@"

# Muted, or the average of all channels (an unbalanced device shows its middle).
get_volume() {
  if [[ "$(pactl "get-$kind-mute" "$dev" 2>/dev/null)" == *yes* ]]; then
    echo Muted
  else
    pactl "get-$kind-volume" "$dev" 2>/dev/null | head -1 \
      | grep -o '[0-9]*%' | awk '{ s += $1; n++ } END { if (n) printf "%d\n", s / n + 0.5 }'
  fi
}

# Every event, plus a made-up "on server" line at start and whenever pactl
# subscribe ends (the sound server restarted): then the volume is read again.
events() {
  while :; do
    echo "Event 'change' on server #0"
    pactl subscribe
    sleep 1
  done
}

# "on server" = the default device changed (e.g. headphones connected): follow it.
events | while read -r line; do
  [[ "$line" == *" $kind "* || "$line" == *" on server "* ]] || continue
  get_volume
done | print_on_change
