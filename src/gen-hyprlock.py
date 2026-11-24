#!/usr/bin/env python3
"""Render the hyprlock background frames and probe image.

    src/gen-hyprlock.py                    # 1920x1080, 6 frames -> hyprlock/
    src/gen-hyprlock.py --size auto        # match your largest monitor
    src/gen-hyprlock.py --seed 42          # different faint stars and dust texture
    src/gen-hyprlock.py --frames 10        # longer twinkle loop
    ./hyprlock/install.sh                  # install the result

Writes stars-0.png ... stars-<N-1>.png and probe.png. The frames are the
wallpaper's sky (voyager_sky.py: Ursa Major and Minor, the Integrated Flux Nebula,
the Owl Nebula and galaxies) without the wallpaper's small probe, with Polaris and
a few faint stars at N evenly spaced points of their twinkle. hyprlock crossfades
to the next frame once a second, and draws the big probe and the widgets on top.

Needs: python3, rsvg-convert (librsvg), magick (ImageMagick).
"""

import argparse
import os
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import ROOT, LOGO, parse_size, rsvg  # noqa: E402
from voyager_sky import Scene, magick, render_base, render_twinkles, PNG_NO_DATES  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--size", default="1920x1080",
                    help="WxH, or 'auto' for the largest connected monitor (default 1920x1080)")
    ap.add_argument("--seed", type=int, default=1977, help="faint stars and dust texture seed (default 1977)")
    ap.add_argument("--frames", type=int, default=6, help="frames, one per second (default 6)")
    ap.add_argument("--out-dir", default=str(ROOT / "hyprlock"), help="output folder (default hyprlock/)")
    a = ap.parse_args()
    if a.frames < 1:
        raise SystemExit("--frames must be at least 1")

    w, h = parse_size(a.size)
    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    scene = Scene(w, h, a.seed)

    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(os.cpu_count() or 4) as pool:
        base = f"{tmp}/base.png"
        render_base(scene, a.seed, base, tmp, pool, with_probe=False)
        layers = [f"{tmp}/t{i}.png" for i in range(a.frames)]
        render_twinkles(scene, a.frames, layers, pool)

        # next-frame.sh cycles through every stars-*.png it finds: drop leftovers
        # from an earlier run with more frames
        for old in out.glob("stars-*.png"):
            old.unlink()
        pngs = [out / f"stars-{i}.png" for i in range(a.frames)]
        for job in [pool.submit(magick, base, layer, "-composite", *PNG_NO_DATES, png)
                    for layer, png in zip(layers, pngs)]:
            job.result()
    for png in pngs:
        print(f"wrote {png}")

    probe = out / "probe.png"
    rsvg(LOGO, 400, 400, probe)
    print(f"wrote {probe}")
    print(f"{w}x{h}, seed {a.seed}, {a.frames} frames")


if __name__ == "__main__":
    main()
