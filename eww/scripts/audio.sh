#!/usr/bin/env bash
# Print the default speaker or mic volume (0-100, or "Muted"), then again on every change.
#
#   audio.sh sink     speaker
#   audio.sh source   microphone
#
# Needs: pactl (libpulse). Errors go to `eww logs`.

kind="$1"
[[ "$kind" == sink || "$kind" == source ]] || { echo "usage: $0 sink|source" >&2; exit 1; }
command -v pactl >/dev/null || { echo "audio.sh: pactl missing — sudo pacman -S libpulse" >&2; exec sleep infinity; }
dev="@DEFAULT_${kind^^}@"

get_volume() {
  if [[ "$(pactl "get-$kind-mute" "$dev")" == *yes* ]]; then
    echo Muted
  else
    pactl "get-$kind-volume" "$dev" | grep -oP '\d+(?=%)' | head -1
  fi
}

get_volume
pactl subscribe | while read -r line; do
  [[ "$line" == *" $kind "* ]] && get_volume
done
