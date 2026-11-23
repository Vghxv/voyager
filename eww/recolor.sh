#!/usr/bin/env bash
# Regenerate eww/icons/ from the black originals in eww/assets/.
#
#   eww/recolor.sh             navy (#14141F), matches the bar text on light cells
#   eww/recolor.sh '#5EEAD4'   any other color
#
# Needs ImageMagick (magick). Keeps each icon's alpha; only the color changes.

set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
COLOR="${1:-#14141F}"

command -v magick >/dev/null || { echo "ImageMagick is not installed (sudo pacman -S imagemagick)." >&2; exit 1; }

mkdir -p "$SRC/icons"
for f in "$SRC"/assets/*.png; do
  # PNG32: so colormap inputs (wifi.png) come out as true RGBA
  magick "$f" -alpha on -fill "$COLOR" -colorize 100 -strip "PNG32:$SRC/icons/$(basename "$f")"
  echo "  $(basename "$f")"
done
echo "Recolored to $COLOR -> $SRC/icons/"
