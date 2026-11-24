#!/usr/bin/env python3
"""Render the hyprlock starfield frames and probe image.

    src/gen-hyprlock.py                    # 1920x1080, 6 frames -> hyprlock/
    src/gen-hyprlock.py --size auto        # match your largest monitor
    src/gen-hyprlock.py --seed 42          # different star layout
    src/gen-hyprlock.py --frames 10        # longer twinkle loop
    ./hyprlock/install.sh                  # install the result

Writes stars-0.png ... stars-<N-1>.png (hyprlock crossfades between them, one
per second) and probe.png. Same gradient as the wallpaper, with a random
starfield; the probe is drawn on top by hyprlock.

Needs: python3, rsvg-convert (librsvg).
"""

import argparse
import random
import sys
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
import voyager_art as art  # noqa: E402

ROOT = art.ROOT


def build_svg(w, h, seed, frame, frames):
    layout = art.make_stars(random.Random(seed), w, h)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{art.BG_TOP}"/><stop offset="1" stop-color="{art.BG_BOTTOM}"/>
  </linearGradient>
  <radialGradient id="star"><stop offset="0" stop-color="#FFFFFF"/>
    <stop offset="0.25" stop-color="#E8E8FF" stop-opacity="0.9"/>
    <stop offset="1" stop-color="#C7CBFF" stop-opacity="0"/></radialGradient>
</defs>
<rect width="{w}" height="{h}" fill="url(#bg)"/>
{art.stars(layout, frame, frames)}
</svg>
"""


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--size", default="1920x1080",
                    help="WxH, or 'auto' for the largest connected monitor (default 1920x1080)")
    ap.add_argument("--seed", type=int, default=1977, help="star layout seed (default 1977)")
    ap.add_argument("--frames", type=int, default=6, help="starfield frames, one per second (default 6)")
    ap.add_argument("--out-dir", default=str(ROOT / "hyprlock"), help="output folder (default hyprlock/)")
    a = ap.parse_args()
    if a.frames < 1:
        raise SystemExit("--frames must be at least 1")

    w, h = art.parse_size(a.size)
    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)

    # next-frame.sh cycles through every stars-*.png it finds: drop leftovers
    # from an earlier run with more frames
    for old in out.glob("stars-*.png"):
        old.unlink()
    for i in range(a.frames):
        png = out / f"stars-{i}.png"
        art.render(build_svg(w, h, a.seed, i, a.frames), w, h, png)
        print(f"wrote {png}")

    probe = out / "probe.png"
    art.rsvg(art.LOGO, 400, 400, probe)
    print(f"wrote {probe}")
    print(f"{w}x{h}, seed {a.seed}, {a.frames} frames")


if __name__ == "__main__":
    main()
