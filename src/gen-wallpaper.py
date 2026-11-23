#!/usr/bin/env python3
"""Render the desktop wallpaper: an animated WebP and a still PNG for awww.

    src/gen-wallpaper.py                       # 1920x1080 -> awww/wallpaper.webp + .png
    src/gen-wallpaper.py --size auto           # match your largest monitor
    src/gen-wallpaper.py --fps 8 --frames 32   # lighter animation, same 4 s loop
    src/gen-wallpaper.py --frames 0            # still PNG only
    src/gen-wallpaper.py --seed 42             # different star layout
    ./awww/install.sh                          # install the result

Gradient, twinkling stars, a faint nebula, and the probe (src/logo.svg) in the
lower-right pulsing signal rings toward a pale blue dot. Nebula placement and
ring pulse are constants below; the palette is in voyager_art.py.

Needs: python3, rsvg-convert (librsvg), magick (ImageMagick, for the WebP).
"""

import argparse
import math
import random
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import (ROOT, LOGO, BG_TOP, BG_BOTTOM, INDIGO, TEAL, DISH,  # noqa: E402
                         make_stars, stars, render, parse_size)


def nebula(w, h):
    blobs = [  # (cx, cy, rx, ry, colour, peak opacity)
        (0.27, 0.30, 0.48, 0.36, INDIGO, 0.10),
        (0.16, 0.48, 0.30, 0.22, TEAL, 0.05),
        (0.40, 0.14, 0.26, 0.16, INDIGO, 0.06),
        (0.86, 0.12, 0.22, 0.18, INDIGO, 0.035),
    ]
    defs, shapes = [], []
    for i, (cx, cy, rx, ry, col, op) in enumerate(blobs):
        defs.append(
            f'<radialGradient id="neb{i}"><stop offset="0" stop-color="{col}" stop-opacity="{op}"/>'
            f'<stop offset="0.55" stop-color="{col}" stop-opacity="{op * 0.35:.3f}"/>'
            f'<stop offset="1" stop-color="{col}" stop-opacity="0"/></radialGradient>')
        shapes.append(f'<ellipse cx="{cx * w:.0f}" cy="{cy * h:.0f}" rx="{rx * w:.0f}" '
                      f'ry="{ry * h:.0f}" fill="url(#neb{i})"/>')
    return "\n".join(defs), "\n".join(shapes)


RING_BASE = (0.45, 0.35, 0.25, 0.15)   # static ring opacities, inner -> outer


def ring_opacity(i, frame, frames):
    """Static: fixed fade. Animated: a pulse travels outward through the rings once per loop."""
    if frames == 1:
        return RING_BASE[i]
    pos = frame / frames * (len(RING_BASE) + 2) - 0.5     # pulse position, in ring units
    return RING_BASE[i] * 0.4 + 0.4 * math.exp(-((i - pos) ** 2) / 0.5)


def probe(w, h, frame=0, frames=1):
    size = 0.09 * w
    px, py = 0.76 * w, 0.66 * h                  # top-left of the logo box
    s = size / 400
    dx, dy = px + DISH[0] * s, py + DISH[1] * s  # dish centre on screen

    body = LOGO.read_text()
    body = re.sub(r"<svg[^>]*>", f'<svg x="{px:.1f}" y="{py:.1f}" width="{size:.1f}" '
                  f'height="{size:.1f}" viewBox="0 0 400 400">', body, count=1)

    # signal rings: arcs opening toward the pale blue dot (upper-left)
    dot = (0.14 * w, 0.22 * h)
    aim = math.atan2(dot[1] - dy, dot[0] - dx)
    arcs = []
    for i, rad in enumerate((0.035, 0.06, 0.088, 0.12)):
        r = rad * w
        span = math.radians(26 - i * 3)
        x1, y1 = dx + r * math.cos(aim - span), dy + r * math.sin(aim - span)
        x2, y2 = dx + r * math.cos(aim + span), dy + r * math.sin(aim + span)
        op = ring_opacity(i, frame, frames)
        sw = 2.2 * w / 1920
        arcs.append(f'<path d="M{x1:.1f},{y1:.1f} A{r:.1f},{r:.1f} 0 0 1 {x2:.1f},{y2:.1f}" fill="none" '
                    f'stroke="{TEAL}" stroke-width="{sw:.2f}" stroke-linecap="round" opacity="{op:.2f}"/>')

    pale = (f'<circle cx="{dot[0]:.1f}" cy="{dot[1]:.1f}" r="{9 * w / 1920:.1f}" fill="url(#pale)"/>'
            f'<circle cx="{dot[0]:.1f}" cy="{dot[1]:.1f}" r="{1.8 * w / 1920:.2f}" fill="#A8D8FF"/>')
    return "\n".join(arcs) + "\n" + pale + "\n" + body


def build_svg(w, h, seed, frame=0, frames=1):
    rng = random.Random(seed)
    layout = make_stars(rng, w, h)
    neb_defs, neb = nebula(w, h)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{BG_TOP}"/><stop offset="1" stop-color="{BG_BOTTOM}"/>
  </linearGradient>
  <radialGradient id="star"><stop offset="0" stop-color="#FFFFFF"/>
    <stop offset="0.25" stop-color="#E8E8FF" stop-opacity="0.9"/>
    <stop offset="1" stop-color="#C7CBFF" stop-opacity="0"/></radialGradient>
  <radialGradient id="pale"><stop offset="0" stop-color="#A8D8FF" stop-opacity="0.55"/>
    <stop offset="1" stop-color="#A8D8FF" stop-opacity="0"/></radialGradient>
  <filter id="grain" x="0" y="0" width="100%" height="100%">
    <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="1" seed="{seed}"/>
    <feColorMatrix type="saturate" values="0"/>
  </filter>
  {neb_defs}
</defs>
<rect width="{w}" height="{h}" fill="url(#bg)"/>
{neb}
<rect width="{w}" height="{h}" filter="url(#grain)" opacity="0.025"/>
{stars(layout, frame, frames)}
{probe(w, h, frame, frames)}
</svg>
"""


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--size", default="1920x1080",
                    help="WxH, or 'auto' for the largest connected monitor (default 1920x1080)")
    ap.add_argument("--seed", type=int, default=1977, help="star layout seed (default 1977)")
    ap.add_argument("--out", default=str(ROOT / "awww" / "wallpaper.png"), help="static PNG (--static)")
    ap.add_argument("--anim", default=str(ROOT / "awww" / "wallpaper.webp"), help="animated WebP")
    ap.add_argument("--frames", type=int, default=48, help="animation frames (default 48, 0 = no animation)")
    ap.add_argument("--fps", type=int, default=12, help="animation frame rate (default 12)")
    ap.add_argument("--quality", type=int, default=85, help="WebP quality (default 85)")
    ap.add_argument("--svg", help="also keep the static SVG here")
    a = ap.parse_args()

    w, h = parse_size(a.size)
    static = build_svg(w, h, a.seed)
    if a.svg:
        Path(a.svg).write_text(static)
    render(static, w, h, a.out)
    print(f"wrote {a.out}")

    if a.frames > 0:
        with tempfile.TemporaryDirectory() as tmp:
            pngs = []
            for i in range(a.frames):
                png = str(Path(tmp) / f"f{i:03d}.png")
                render(build_svg(w, h, a.seed, i, a.frames), w, h, png)
                pngs.append(png)
            delay = f"{100 / a.fps:.2f}"            # ImageMagick delay is in 1/100 s
            try:
                subprocess.run(["magick", "-delay", delay, *pngs, "-loop", "0",
                                "-quality", str(a.quality), a.anim], check=True)
            except FileNotFoundError:
                raise SystemExit("magick not found (needed for the animated WebP). "
                                 "Install it: sudo pacman -S imagemagick, or use --frames 0")
        print(f"wrote {a.anim} ({a.frames} frames @ {a.fps} fps)")
    print(f"{w}x{h}, seed {a.seed}")


if __name__ == "__main__":
    main()
