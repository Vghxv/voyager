#!/usr/bin/env python3
"""Render the Plymouth theme images: the artwork SVGs in src/art/ and the voyager sky.

    src/gen-plymouth.py                 # 1920x1080 -> plymouth/voyager/*.png
    src/gen-plymouth.py --size auto     # match your largest monitor (sharpest)
    src/gen-plymouth.py --seed 42       # different faint stars and dust texture
    sudo ./plymouth/install.sh --apply  # install and rebuild the initramfs

Artwork, each at its SVG's own size: logo (the probe), ring (signal ring), star,
streak (warp streak), flash (wrong-passphrase flash, stretched).

The sky is voyager_sky.py's Big Dipper field:
- sky.png        the sky with every star at rest, except Polaris;
- polaris.png    Polaris alone, cropped: the script makes it breathe. Its place in
                 sky.png is written into voyager.script (POLARIS_X / POLARIS_Y);
- blueshift.png, redshift.png   a share of the background stars, Doppler-shifted, on
                 a transparent layer: faded in as the probe speeds up or backs off.

Needs: python3, rsvg-convert (librsvg), magick (ImageMagick).
"""


import argparse
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import ROOT, magick, parse_size, render, rsvg, sky_args  # noqa: E402
from voyager_sky import (Scene, flatten, render_base, workspace,  # noqa: E402
                         PNG_NO_DATES, BLUE_SHIFT, RED_SHIFT)

ASSETS = ("logo", "ring", "star", "streak", "flash")
SCRIPT = ROOT / "plymouth" / "voyager" / "voyager.script"


def render_sky(w, h, seed, out):
    """Write the sky layers into out; return Polaris's place as fractions of w, h."""
    scene = Scene(w, h, seed)
    px, py, pw, ph = scene.polaris_box()
    with workspace() as (tmp, pool):
        base, rest, star = f"{tmp}/base.png", f"{tmp}/rest.png", f"{tmp}/polaris.png"
        jobs = [pool.submit(render, svg, w, h, png) for svg, png in (
            (scene.twinkle_layer(with_polaris=False), rest),
            (scene.polaris_layer(), star),
            (scene.shift_layer(BLUE_SHIFT), out / "blueshift.png"),
            (scene.shift_layer(RED_SHIFT), out / "redshift.png"))]
        render_base(scene, seed, base, tmp, pool)
        for job in jobs:
            job.result()
        flatten(base, rest, out / "sky.png")
        magick(star, "-crop", f"{pw}x{ph}+{px}+{py}", "+repage", *PNG_NO_DATES, out / "polaris.png")
    return px / w, py / h


def write_polaris(fx, fy):
    """Put Polaris's place in sky.png into voyager.script, between its markers."""
    text = SCRIPT.read_text()
    new, n = re.subn(r"(POLARIS_X\s*=\s*)[\d.]+(;.*\n\s*POLARIS_Y\s*=\s*)[\d.]+",
                     rf"\g<1>{fx:.6f}\g<2>{fy:.6f}", text)
    if n != 1:
        raise SystemExit(f"{SCRIPT}: no POLARIS_X / POLARIS_Y lines to update")
    if new != text:
        SCRIPT.write_text(new)
        print(f"updated {SCRIPT} (Polaris at {fx:.4f}, {fy:.4f})")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sky_args(ap)
    ap.add_argument("--out-dir", default=str(ROOT / "plymouth" / "voyager"),
                    help="output folder (default plymouth/voyager/)")
    a = ap.parse_args()

    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    for name in ASSETS:
        svg = ROOT / "src" / "art" / f"{name}.svg"
        m = re.search(r'<svg[^>]*\bwidth="(\d+)"[^>]*\bheight="(\d+)"', svg.read_text())
        if not m:
            raise SystemExit(f"{svg}: no width/height on <svg>")
        w, h = m.groups()
        png = out / f"{name}.png"
        rsvg(svg, w, h, png)
        print(f"wrote {png} ({w}x{h})")

    w, h = parse_size(a.size)
    fx, fy = render_sky(w, h, a.seed, out)
    for name in ("sky", "polaris", "blueshift", "redshift"):
        print(f"wrote {out / name}.png")
    write_polaris(fx, fy)
    print(f"{w}x{h}, seed {a.seed}")


if __name__ == "__main__":
    main()
