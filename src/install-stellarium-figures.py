#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["numpy", "pillow"]
# ///
"""Drag mode's constellation figures from Stellarium's modern illustrations.

    src/install-stellarium-figures.py           # -> awww/sky/figures/{Name}.png and frames.json
    src/install-stellarium-figures.py --cache DIR   # where downloads are kept (default ~/.cache/voyager-sky)

Stellarium's modern sky culture (the same one src/build-sky-data.py takes the
stick figures from) ships a drawing for each constellation by Johan Meuris
(Free Art License 1.3), drawn over the real stars. Its index.json pins each
image to the sky by three anchor stars: image (x, y, 1) maps linearly to the
anchors' unit vectors, then onto the sphere. This maps each drawing into the
frame drag mode draws figures in (a square of the tangent plane at the
mean of the line stars, east left, north up), tints its grey to the
constellation's colour with the brightness as transparency, and writes a
1024 x 1024 RGBA PNG per constellation. Run it after src/build-sky-data.py:
the frames come from the stick figures in awww/sky/constellations.json.

frames.json is where drag mode (awww/voyager-sky/src/data.rs) reads each
figure's placement from, by abbreviation: its file, the frame's centre (a unit
vector) and half-width in tangent units, as wide as the drawing needs and at
least FIGURE_MARGIN x the farthest line star. Argo Navis is one drawing
(Carina's), shown for Puppis and Vela too; Ophiuchus's drawing holds the
serpent and is shown for Serpens.

Downloads (src/sky_sources.py) are kept in the same cache as build-sky-data.py's:
the sky culture's index, HYG, and the drawings in illustrations/ (about 3 MB, once).

Needs: uv (which fetches numpy and Pillow, per the header above), curl.
"""

import argparse
import json
import sys
import unicodedata
from pathlib import Path

import numpy as np
from PIL import Image

sys.dont_write_bytecode = True          # no src/__pycache__
from sky_sources import MODERN, cache_arg, fetch, hyg_stars  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SHARED = {"Pup": "Car", "Vel": "Car", "Ser": "Oph"}      # drawings that hold more than one constellation
LIT = 0.05              # grey above this is drawing, not background
PAD = 1.03              # frame half-width over the drawing's farthest lit pixel
OPACITY = 0.55          # the drawings are filled bodies, not lines: strong, but the stars still show through
EVEN = 0.78             # darker tints (sapphire, magenta) get more opacity, up to 1.5x, to look as strong
COLOURS = {             # the figures' tints, by name
    "magenta-pink": (255, 92, 200), "electric cyan": (34, 229, 255), "warm gold": (255, 194, 77),
    "lavender violet": (181, 156, 255), "soft emerald green": (94, 230, 160), "aqua teal": (94, 234, 212),
    "cool silver-white": (220, 228, 245), "bright sapphire blue": (77, 141, 255),
}
COLOUR = {
    "And": "magenta-pink", "Ant": "cool silver-white", "Aps": "lavender violet", "Aql": "electric cyan",
    "Aqr": "warm gold", "Ara": "electric cyan", "Ari": "warm gold", "Aur": "magenta-pink",
    "Boo": "soft emerald green", "CMa": "bright sapphire blue", "CMi": "bright sapphire blue",
    "CVn": "soft emerald green", "Cae": "cool silver-white", "Cam": "soft emerald green", "Cap": "warm gold",
    "Car": "aqua teal", "Cas": "magenta-pink", "Cen": "electric cyan", "Cep": "magenta-pink",
    "Cet": "magenta-pink", "Cha": "lavender violet", "Cir": "cool silver-white", "Cnc": "warm gold",
    "Col": "aqua teal", "Com": "soft emerald green", "CrA": "electric cyan", "CrB": "soft emerald green",
    "Crt": "electric cyan", "Cru": "electric cyan", "Crv": "electric cyan", "Cyg": "electric cyan",
    "Del": "aqua teal", "Dor": "aqua teal", "Dra": "soft emerald green", "Equ": "aqua teal",
    "Eri": "aqua teal", "For": "cool silver-white", "Gem": "warm gold", "Gru": "lavender violet",
    "Her": "electric cyan", "Hor": "cool silver-white", "Hya": "electric cyan", "Hyi": "lavender violet",
    "Ind": "lavender violet", "LMi": "soft emerald green", "Lac": "magenta-pink", "Leo": "warm gold",
    "Lep": "bright sapphire blue", "Lib": "warm gold", "Lup": "electric cyan", "Lyn": "soft emerald green",
    "Lyr": "electric cyan", "Men": "cool silver-white", "Mic": "cool silver-white",
    "Mon": "bright sapphire blue", "Mus": "lavender violet", "Nor": "cool silver-white",
    "Oct": "cool silver-white", "Oph": "electric cyan", "Ori": "bright sapphire blue",
    "Pav": "lavender violet", "Peg": "magenta-pink", "Per": "magenta-pink", "Phe": "lavender violet",
    "Pic": "cool silver-white", "PsA": "aqua teal", "Psc": "warm gold", "Pup": "aqua teal",
    "Pyx": "aqua teal", "Ret": "cool silver-white", "Scl": "cool silver-white", "Sco": "warm gold",
    "Sct": "electric cyan", "Ser": "electric cyan", "Sex": "cool silver-white", "Sge": "electric cyan",
    "Sgr": "warm gold", "Tau": "warm gold", "Tel": "cool silver-white", "TrA": "electric cyan",
    "Tri": "magenta-pink", "Tuc": "lavender violet", "UMa": "soft emerald green", "UMi": "soft emerald green",
    "Vel": "aqua teal", "Vir": "warm gold", "Vol": "lavender violet", "Vul": "electric cyan",
}

SKY = ROOT / "awww" / "sky" / "constellations.json"
OUT = ROOT / "awww" / "sky" / "figures"
SIZE = 1024
FIGURE_MARGIN = 1.4     # a frame reaches at least this many times its farthest line star
FLOOR = 10 / 255        # below this a pixel is fully transparent


def unit(ra, dec):
    a, d = np.radians(ra), np.radians(dec)
    return np.stack([np.cos(d) * np.cos(a), np.cos(d) * np.sin(a), np.sin(d)], -1)


def frame(stars, lines):
    """The figure's frame: centre, east, north and R (the farthest line star, in tangent units)."""
    idx = sorted({i for l in lines for i in l})
    p = unit(np.array([stars[i][0] for i in idx]), np.array([stars[i][1] for i in idx]))
    c = p.sum(0)
    c /= np.linalg.norm(c)
    e = np.array([-c[1], c[0], 0.0])
    e /= np.linalg.norm(e)
    n = np.cross(c, e)
    d = p @ c
    r = np.hypot(p @ e / d, p @ n / d).max()
    return c, e, n, max(r, 1e-3)


def bilinear(img, x, y):
    h, w = img.shape
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    fx, fy = x - x0, y - y0
    out = np.zeros(x.shape, np.float32)
    for dx, dy, wgt in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)), (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
        xi, yi = x0 + dx, y0 + dy
        ok = (xi >= 0) & (xi < w) & (yi >= 0) & (yi < h)
        out += np.where(ok, img[np.clip(yi, 0, h - 1), np.clip(xi, 0, w - 1)], 0) * wgt
    return out


def to_rgba(rgb):
    """Full-brightness hue, brightness as alpha; transparent pixels carry no colour (smaller files)."""
    m = rgb.max(-1)
    a = np.clip((m - FLOOR) / (1 - FLOOR), 0, 1)
    col = np.where(a[..., None] > 0, rgb / np.maximum(m, 1e-6)[..., None], 0)
    return Image.fromarray((np.concatenate([col, a[..., None]], -1) * 255 + 0.5).astype(np.uint8), "RGBA")


def figure_name(en):
    """A figure's file name: the full English name, ASCII, spaces as underscores (Boötes -> Bootes)."""
    return unicodedata.normalize("NFKD", en).encode("ascii", "ignore").decode().replace(" ", "_")


def sky_map(image, pos, width):
    """3x3 M with sky ~ M (x, y, 1), x and y in pixels of the image as stored (Stellarium's anchors)."""
    sx = width / image["size"][0]
    a = np.stack([unit(*pos[k["hip"]]) for k in image["anchors"]], 1)
    b = np.array([[k["pos"][0] * sx for k in image["anchors"]],
                  [k["pos"][1] * sx for k in image["anchors"]], [1.0, 1.0, 1.0]])
    return a @ np.linalg.inv(b)


def reach(grey, m, c, e, n):
    """How far the drawing reaches from c, in tangent units (the larger of east and north)."""
    ys, xs = np.nonzero(grey > LIT)
    p = (m @ np.stack([xs + 0.5, ys + 0.5, np.ones(len(xs))])).T
    p /= np.linalg.norm(p, axis=1, keepdims=True)
    d = p @ c
    return float(np.maximum(np.abs(p @ e / d), np.abs(p @ n / d)).max())


def warp(grey, m, c, e, n, half):
    t = (np.arange(SIZE) + 0.5) / SIZE
    u, v = np.meshgrid(t, t)
    x, y = (0.5 - u) * 2 * half, (0.5 - v) * 2 * half
    p = c + x[..., None] * e + y[..., None] * n
    q = p @ np.linalg.inv(m).T
    ok = q[..., 2] > 0
    w = np.where(ok, q[..., 2], 1)
    out = bilinear(grey, q[..., 0] / w - 0.5, q[..., 1] / w - 0.5)
    return np.where(ok, out, 0)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    cache_arg(ap)
    args = ap.parse_args()

    got = fetch(args.cache, ["modern.json", "hyg.csv"])
    mod = json.loads(got["modern.json"].read_text())
    images = {c["id"].split()[-1]: c["image"] for c in mod["constellations"] if "image" in c}
    drawings = {f"illustrations/{Path(i['file']).name}": f"{MODERN}/{i['file']}" for i in images.values()}
    art = fetch(args.cache, sorted(drawings), drawings)
    pos = {hip: s[:2] for hip, s in hyg_stars(got["hyg.csv"])[0].items()}
    sky = json.loads(SKY.read_text())
    OUT.mkdir(parents=True, exist_ok=True)
    frames, ratios, greys = {}, [], {}
    for con in sky["constellations"]:
        abbr = con["abbr"]
        image = images[SHARED.get(abbr, abbr)]
        if image["file"] not in greys:      # Argo and Ophiuchus serve more than one constellation
            src = Image.open(art[f"illustrations/{Path(image['file']).name}"]).convert("L")
            if src.width < SIZE:            # 128-512 px drawings: smooth edges before the warp enlarges them
                src = src.resize((SIZE, SIZE * src.height // src.width), Image.LANCZOS)
            greys[image["file"]] = np.asarray(src, np.float32) / 255
        grey = greys[image["file"]]
        m = sky_map(image, pos, grey.shape[1])
        c, e, n, r = frame(sky["stars"], con["lines"])
        half = max(FIGURE_MARGIN * r, PAD * reach(grey, m, c, e, n))
        a = warp(grey, m, c, e, n, half)
        tint = np.array(COLOURS[COLOUR[abbr]], np.float32) / 255
        lum = float(tint @ np.array([0.2126, 0.7152, 0.0722], np.float32))
        rgb = min(1.0, OPACITY * min(1.5, max(1.0, EVEN / lum))) * a[..., None] * tint
        name = f"{figure_name(con['en'])}.png"
        to_rgba(rgb).save(OUT / name, optimize=True)
        frames[abbr] = {"file": name, "centre": [round(float(x), 9) for x in c], "half": round(half, 9)}
        ratios.append((half / r, abbr))
    rows = ",\n".join(f" {json.dumps(k)}: {json.dumps(frames[k])}" for k in sorted(frames))
    (OUT / "frames.json").write_text("{\n" + rows + "\n}\n")     # one figure per line
    ratios.sort(reverse=True)
    print(f"wrote {len(frames)} figures and frames.json to {OUT}")
    print("widest frames (half / farthest line star):", ", ".join(f"{a} {x:.2f}" for x, a in ratios[:8]))


if __name__ == "__main__":
    main()
