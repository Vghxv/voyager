#!/usr/bin/env python3
"""Render the Plymouth theme images from their SVGs in src/.

    src/gen-plymouth.py                 # -> plymouth/voyager/*.png
    sudo ./plymouth/install.sh --apply  # install and rebuild the initramfs

Each PNG is rendered at its SVG's own size: logo (the probe), ring (signal
ring), star, streak (warp streak), flash (wrong-passphrase flash, stretched).

Needs: python3, rsvg-convert (librsvg).
"""


import argparse
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import ROOT, rsvg  # noqa: E402

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
        rsvg(svg, w, h, png)
        print(f"wrote {png} ({w}x{h})")


if __name__ == "__main__":
    main()
