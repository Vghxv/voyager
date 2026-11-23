#!/usr/bin/env python3
"""Render the voyager hyprlock assets (SVG -> PNG via rsvg-convert).

Writes the starfield frames hyprlock crossfades between (stars-0.png ...
stars-<N-1>.png, picked by hyprlock/next-frame.sh) and the probe image
(probe.png, src/logo.svg at 400x400). The background gradient and stars come
from src/gen-wallpaper.py, without the nebula and the probe, because hyprlock
draws the probe and the password field on top.

    src/gen-hyprlock.py                       # 1920x1080, 6 frames -> hyprlock/
    src/gen-hyprlock.py --size 2560x1440      # other monitor size
    src/gen-hyprlock.py --size auto           # your largest monitor
    src/gen-hyprlock.py --seed 42             # different star layout
    ./hyprlock/install.sh                     # install the new files

The frame count must match next-frame.sh (`% 6`). Python stdlib only; needs
rsvg-convert (librsvg).
"""

import argparse
import importlib.util
import random
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FRAMES = 6

# reuse the wallpaper's palette, star layout and renderer
sys.dont_write_bytecode = True          # no src/__pycache__ from this import
_spec = importlib.util.spec_from_file_location("gen_wallpaper", ROOT / "src" / "gen-wallpaper.py")
wp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(wp)


def build_svg(w, h, seed, frame):
    layout = wp.make_stars(random.Random(seed), w, h)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{wp.BG_TOP}"/><stop offset="1" stop-color="{wp.BG_BOTTOM}"/>
  </linearGradient>
  <radialGradient id="star"><stop offset="0" stop-color="#FFFFFF"/>
    <stop offset="0.25" stop-color="#E8E8FF" stop-opacity="0.9"/>
    <stop offset="1" stop-color="#C7CBFF" stop-opacity="0"/></radialGradient>
</defs>
<rect width="{w}" height="{h}" fill="url(#bg)"/>
{wp.stars(layout, frame, FRAMES)}
</svg>
"""


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--size", default="1920x1080",
                    help="WxH, or 'auto' for the largest connected monitor (default 1920x1080)")
    ap.add_argument("--seed", type=int, default=1977, help="star layout seed (default 1977)")
    ap.add_argument("--out-dir", default=str(ROOT / "hyprlock"), help="output folder (default hyprlock/)")
    a = ap.parse_args()

    w, h = wp.parse_size(a.size)
    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)

    for i in range(FRAMES):
        png = out / f"stars-{i}.png"
        wp.render(build_svg(w, h, a.seed, i), w, h, str(png))
        print(f"wrote {png}")

    probe = out / "probe.png"
    subprocess.run(["rsvg-convert", "-w", "400", "-h", "400", str(wp.LOGO), "-o", str(probe)], check=True)
    print(f"wrote {probe}")
    print(f"{w}x{h}, seed {a.seed}, {FRAMES} frames")


if __name__ == "__main__":
    main()
