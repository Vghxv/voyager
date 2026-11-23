#!/usr/bin/env python3
"""Render the voyager Plymouth theme images (SVG -> PNG via rsvg-convert).

Each image is rendered from its SVG in src/ at the SVG's own size:

    logo.png    src/logo.svg     400x400  the probe
    ring.png    src/ring.svg     512x512  signal ring from the dish
    star.png    src/star.svg      24x24   starfield star
    streak.png  src/streak.svg    80x10   warp streak
    flash.png   src/flash.svg      4x4    wrong-passphrase flash, scaled to the screen

    src/gen-plymouth.py                     # -> plymouth/voyager/
    ./plymouth/install.sh                   # install the theme and rebuild the initramfs

If the probe artwork moves its dish, update dish_x / dish_y in voyager.script.
Python stdlib only; needs rsvg-convert (librsvg).
"""

import argparse
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ("logo", "ring", "star", "streak", "flash")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out-dir", default=str(ROOT / "plymouth" / "voyager"),
                    help="output folder (default plymouth/voyager/)")
    a = ap.parse_args()

    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    for name in ASSETS:
        svg = ROOT / "src" / f"{name}.svg"
        m = re.search(r'<svg[^>]*\bwidth="(\d+)"[^>]*\bheight="(\d+)"', svg.read_text())
        if not m:
            raise SystemExit(f"{svg}: no width/height on <svg>")
        w, h = m.groups()
        png = out / f"{name}.png"
        subprocess.run(["rsvg-convert", "-w", w, "-h", h, str(svg), "-o", str(png)], check=True)
        print(f"wrote {png} ({w}x{h})")


if __name__ == "__main__":
    main()
