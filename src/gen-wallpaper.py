#!/usr/bin/env python3
"""Render the desktop wallpaper: an animated WebP and a still PNG for awww.

    src/gen-wallpaper.py                       # 1920x1080 -> awww/wallpaper.webp + .png
    src/gen-wallpaper.py --size auto           # match your largest monitor (sharpest)
    src/gen-wallpaper.py --fps 8 --frames 32   # lighter animation, same 4 s loop
    src/gen-wallpaper.py --frames 0            # still PNG only
    src/gen-wallpaper.py --seed 42             # different faint stars and dust texture
    ./awww/install.sh                          # install the result

Ursa Major and Ursa Minor in the middle of the screen, laid out as in a wide-field
photo of the two (the Big Dipper's handle up on the left, its pointers aiming right
at Polaris, the end of the Little Dipper's handle). Each star is drawn by its
magnitude and its true colour (B-V index -> temperature -> blackbody RGB).

Everything around them is placed from real sky coordinates: the stars' catalogue
positions are fitted to the photo (a gnomonic projection, < 1 px error), and the
same fit puts the deep-sky objects where a long exposure of this field shows them:
- the Integrated Flux Nebula: faint grey-brown dust lit by the whole Milky Way,
  thickening toward lower galactic latitude (the right of the screen), with the
  Polaris Flare beside Polaris and the cloud around M81/M82;
- the Owl Nebula (M97), a teal planetary nebula next to Merak;
- the galaxies M81, M82, M101, M108, M109 and NGC 6217, at true size and angle.
The faint background stars also thicken toward the galactic plane. A tiny probe
sits in the lower right. In the WebP only Polaris and a few faint background stars
twinkle.

The sky itself is drawn by voyager_sky.py (shared with the lock screen). Here, the
still base and the per-frame twinkle layers become a still PNG and a lossless WebP
(lossy WebP blocks up the dark gradient); ffmpeg overlays the layers on the base
while it encodes.

Needs: python3, rsvg-convert (librsvg), magick (ImageMagick). ffmpeg is used for
the WebP when installed (about 4x smaller and much faster); ImageMagick otherwise.
"""

import argparse
import os
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import ROOT, parse_size  # noqa: E402
from voyager_sky import Scene, magick, render_base, render_twinkles, PNG_NO_DATES  # noqa: E402


# --- output ------------------------------------------------------------------------

def encode_webp(base, layers, pattern, fps, out):
    """Lossless animated WebP: each twinkle layer over the still base.

    ffmpeg composites while it encodes and stores each frame as just the pixels that
    changed (~1.5 MB). Without it, ImageMagick composites each frame and encodes
    (re-storing the whole changed rectangle every frame: ~4x larger, much slower)."""
    try:
        subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                        "-loop", "1", "-framerate", str(fps), "-i", base,   # base repeated per frame
                        "-framerate", str(fps), "-i", pattern,
                        "-filter_complex", "[0][1]overlay=format=rgb:shortest=1",
                        "-c:v", "libwebp_anim", "-lossless", "1", "-compression_level", "6",
                        "-loop", "0", "-pix_fmt", "bgra", out], check=True)
        return
    except FileNotFoundError:
        print("ffmpeg not found, encoding with ImageMagick (bigger, slower). "
              "For the small one: sudo pacman -S ffmpeg", file=sys.stderr)
    args = [base, "-write", "mpr:base", "+delete"]
    for layer in layers:
        args += ["(", "mpr:base", layer, "-composite", ")"]
    magick("-delay", f"{100 / fps:.2f}", *args, "-loop", 0,       # delay is in 1/100 s
           "-define", "webp:lossless=true", "-define", "webp:method=6", out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--size", default="1920x1080",
                    help="WxH, or 'auto' for the largest connected monitor (default 1920x1080)")
    ap.add_argument("--seed", type=int, default=1977, help="faint stars and dust texture seed (default 1977)")
    ap.add_argument("--out", default=str(ROOT / "awww" / "wallpaper.png"), help="static PNG (--static)")
    ap.add_argument("--anim", default=str(ROOT / "awww" / "wallpaper.webp"), help="animated WebP")
    ap.add_argument("--frames", type=int, default=48, help="animation frames (default 48, 0 = no animation)")
    ap.add_argument("--fps", type=int, default=12, help="animation frame rate (default 12)")
    ap.add_argument("--svg", help="also keep the still layer's SVG here")
    a = ap.parse_args()

    w, h = parse_size(a.size)
    scene = Scene(w, h, a.seed)
    if a.svg:
        Path(a.svg).write_text(scene.still_layer())

    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(os.cpu_count() or 4) as pool:
        base, rest = f"{tmp}/base.png", f"{tmp}/rest.png"
        render_base(scene, a.seed, base, tmp, pool)
        render_twinkles(scene, 1, [rest], pool)
        magick(base, rest, "-composite", *PNG_NO_DATES, a.out)
        print(f"wrote {a.out}")

        if a.frames > 0:
            layers = [f"{tmp}/t{i:03d}.png" for i in range(a.frames)]
            render_twinkles(scene, a.frames, layers, pool)
            encode_webp(base, layers, f"{tmp}/t%03d.png", a.fps, a.anim)
            print(f"wrote {a.anim} ({a.frames} frames @ {a.fps} fps, lossless)")
    print(f"{w}x{h}, seed {a.seed}")


if __name__ == "__main__":
    main()
