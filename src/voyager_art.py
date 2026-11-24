"""Shared building blocks for the voyager image generators.

Imported by the generators and by voyager_sky.py: paths, the palette, the probe's
dish position, rendering and --size parsing. Not a script.
"""

import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "src" / "logo.svg"

BG_TOP, BG_BOTTOM = "#14141F", "#05050D"   # Plymouth Window.SetBackground*Color
INDIGO, TEAL, MIST = "#8B8FF0", "#5EEAD4", "#D9D9F2"
# Dish centre in the 400x400 logo. plymouth/voyager/voyager.script has its own
# copy (dish_x / dish_y): update both if the probe artwork changes.
DISH = (190, 150)


def render(svg, w, h, out):
    """SVG text -> PNG file at w x h."""
    with tempfile.NamedTemporaryFile("w", suffix=".svg") as f:
        f.write(svg)
        f.flush()
        rsvg(f.name, w, h, out)


def rsvg(svg_path, w, h, out):
    """SVG file -> PNG file, with a readable error when rsvg-convert is missing."""
    try:
        subprocess.run(["rsvg-convert", "-w", str(w), "-h", str(h), str(svg_path), "-o", str(out)],
                       check=True)
    except FileNotFoundError:
        raise SystemExit("rsvg-convert not found. Install it: sudo pacman -S librsvg")


def parse_size(size):
    """'WxH', or 'auto' for the largest connected monitor (hyprctl), rotation included."""
    if size.lower() != "auto":
        try:
            w, h = (int(v) for v in size.lower().split("x"))
        except ValueError:
            raise SystemExit(f"--size {size!r}: expected WxH (e.g. 2560x1440) or auto")
        return w, h
    import json
    try:
        mons = json.loads(subprocess.run(["hyprctl", "monitors", "-j"], capture_output=True,
                                         text=True, check=True).stdout)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as e:
        raise SystemExit(f"--size auto needs a running Hyprland (hyprctl monitors -j): {e}")
    if not mons:
        raise SystemExit("--size auto: hyprctl reports no monitors")
    # transforms 1, 3, 5, 7 rotate by 90 degrees and swap width and height
    sizes = [(m["height"], m["width"]) if m.get("transform", 0) % 2 else (m["width"], m["height"])
             for m in mons]
    return max(sizes, key=lambda wh: wh[0] * wh[1])
