"""Shared building blocks for the voyager image generators.

Imported by gen-wallpaper.py and gen-hyprlock.py, so the wallpaper and the lock
screen use the same palette and the same starfield. Not a script.
"""

import math
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


def make_stars(rng, w, h):
    """Star layout, fixed for all frames: (x, y, r, opacity, twinkle, phase, glow)."""
    n = int(360 * (w * h) / (1920 * 1080))
    out = []
    for _ in range(n):
        x, y = rng.uniform(0, w), rng.uniform(0, h)
        k = rng.random() ** 3                   # most stars small and faint
        r = (1.2 + k * 6.5) * w / 1920
        a = 0.25 + 0.75 * rng.random() ** 1.5
        # twinkle speed: 1 or 2 cycles per loop, so the animation loops seamlessly
        twinkle = rng.choice((1, 2)) if rng.random() < 0.4 else 0
        out.append((x, y, r, a, twinkle, rng.uniform(0, 2 * math.pi), True))
    # a few crisp pin-points
    for _ in range(n // 4):
        x, y = rng.uniform(0, w), rng.uniform(0, h)
        out.append((x, y, 0.6 * w / 1920, rng.uniform(0.3, 0.8), 0, 0, False))
    return out


def stars(layout, frame, frames):
    """SVG circles for one frame of the layout (expects a `star` gradient in <defs>)."""
    out = []
    t = 2 * math.pi * frame / frames
    for x, y, r, a, twinkle, phase, glow in layout:
        if twinkle and frames > 1:
            a *= 0.6 + 0.4 * math.cos(twinkle * t + phase)
        fill = "url(#star)" if glow else MIST
        out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.2f}" fill="{fill}" opacity="{a:.2f}"/>')
    return "\n".join(out)


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
