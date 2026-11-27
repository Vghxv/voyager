#!/usr/bin/env python3
"""Build the sky data the drag-mode wallpaper (awww/voyager-sky) draws.

    src/sky/build-sky-data.py                  # -> awww/sky/{constellations.json,field.bin,milkyway.png,probe.png}
    src/sky/build-sky-data.py --seed 42        # different faint background stars
    src/sky/build-sky-data.py --cache DIR      # where downloads are kept (default ~/.cache/voyager-sky)

Run once; the results are committed. Then src/sky/install-stellarium-figures.py
makes the figures from the new constellations.json. Voyager's own position
comes from src/sky/query-horizons.sh (awww/sky/voyager1.json, voyager2.json).

Sources (src/sky/tables/sources; downloaded once into the cache, never into the repo):
- Stellarium's "modern" sky culture: the 88 IAU constellations and their stick
  figures as Hipparcos numbers (CC BY-SA 4.0). The Traditional Chinese names are
  Taiwan's, in src/sky/tables/constellations.
- HYG database v4.1 (CC BY-SA 4.0): each star's position, V magnitude and
  B-V colour index (from Hipparcos), and its Bayer/Flamsteed designation.
- Roman (1987), CDS VI/42: the IAU constellation boundaries, via astropy's copy.
- NASA SVS Deep Star Maps 2020 (starmap_2020_4k.exr): the whole sky in ICRF
  plate carree. A median filter removes its stars and leaves the diffuse
  Milky Way (and the Magellanic Clouds), in their true place.

Writes:
- constellations.json  {"stars": [[ra, dec, V, r, g, b]...],
                        "constellations": [{abbr, en, zh, lines: [[i, j]...], label: [ra, dec],
                                            members: [i...]}...]}
                        ra/dec in degrees (J2000), r/g/b 0-255 from B-V (voyager_art.bv_rgb).
                        The stars are the stick figures' stars and every star HYG gives a
                        Bayer (Greek letter) or Flamsteed designation, brighter than FAINTEST.
                        Each star belongs to exactly one constellation, the one whose IAU
                        boundary holds it (Roman 1987), and is in that one's `members`, its
                        own stick-figure stars first. Figures still borrow a few neighbours'
                        stars in `lines` (Alpheratz, Elnath); the build lists them, and the
                        stars named for one constellation that lie in another (rho Aql).
- field.bin            faint background stars, 16 bytes each, little-endian:
                        f32 ra, f32 dec, u8 r, g, b, alpha, f32 size (px at 1080p).
                        Seeded and fixed on the sphere, denser toward the galactic plane.
- milkyway.png         2048x1024 8-bit RGB, RA 0 at the centre increasing to the left
                        (as the SVS map), Dec +90 at the top; value = sqrt(linear / max).
- probe.png            src/art/logo.svg at 256 px.

Needs: python3, curl, magick (ImageMagick with OpenEXR), rsvg-convert.
"""

import argparse
import json
import math
import random
import re
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True          # no __pycache__ in src/
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))   # voyager_art, in src/
from voyager_art import ROOT, LOGO, bv_rgb, eq2gal, field_bv, magick, rsvg, star_density, unit  # noqa: E402
from sky_sources import SOURCES, cache_arg, fetch, hyg_stars, table  # noqa: E402

OUT = ROOT / "awww" / "sky"
ZH = {abbr: zh for abbr, zh, *_ in table("constellations")}   # Taiwan's Chinese names

FAINTEST = 6.0               # only designated stars brighter than this are drawn (stick-figure stars always are)
FIELD_STARS = 60000          # whole sky; a 100-degree view shows ~8000
MILKY_WAY_SIZE = "2048x1024"


def radec(v):
    x, y, z = v
    n = math.sqrt(x * x + y * y + z * z)
    return math.degrees(math.atan2(y, x)) % 360, math.degrees(math.asin(z / n))


def hex_rgb(col):
    return [int(col[i:i + 2], 16) for i in (1, 3, 5)]


def largest_part(members, segs):
    """The biggest connected piece of a figure: Serpens comes in two, and its label
    belongs on one of them, not in Ophiuchus between them."""
    parent = {m: m for m in members}

    def root(m):
        while parent[m] != m:
            m = parent[m]
        return m
    for a, b in segs:
        parent[root(a)] = root(b)
    parts = {}
    for m in members:
        parts.setdefault(root(m), []).append(m)
    return max(parts.values(), key=len)


B1875 = (2405889.258550475 - 2451545.0) / 36525     # Julian centuries from J2000
# IAU 1976 precession angles (Lieske 1977) from J2000 back to B1875, in radians
ZETA, Z, THETA = (math.radians(x / 3600) for x in (
    2306.2181 * B1875 + 0.30188 * B1875 ** 2 + 0.017998 * B1875 ** 3,
    2306.2181 * B1875 + 1.09468 * B1875 ** 2 + 0.018203 * B1875 ** 3,
    2004.3109 * B1875 - 0.42665 * B1875 ** 2 - 0.041833 * B1875 ** 3))


def iau_boundaries(path):
    """Roman (1987): rows of (RA low h, RA high h, Dec low deg, abbr), north to south.
    The table rounds to 1e-4 h, up to 0.18 s of time: Delporte's edges are whole seconds
    and whole arcminutes, so snap back to them (the Aql/Del edge at 20h08m30s is stored
    as 20.1417 h, 1.7" east, which kept rho Aql in Aquila until 2028)."""
    rows = []
    for line in path.read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            lo, hi, dec, abbr = line.split()
            rows.append((round(float(lo) * 3600) / 3600, round(float(hi) * 3600) / 3600,
                         round(float(dec) * 60) / 60, abbr))
    return rows


def iau_constellation(rows, ra, dec):
    """The one constellation (IAU, 1930) a J2000 position lies in: precess it to B1875
    (IAU 1976 precession, Lieske 1977), then take the first box below it (Roman 1987)."""
    a, d = math.radians(ra) + ZETA, math.radians(dec)
    x = math.cos(THETA) * math.cos(d) * math.cos(a) - math.sin(THETA) * math.sin(d)
    y = math.cos(d) * math.sin(a)
    w = math.sin(THETA) * math.cos(d) * math.cos(a) + math.cos(THETA) * math.sin(d)
    ra_h = math.degrees(math.atan2(y, x) + Z) % 360 / 15
    dec_b = math.degrees(math.asin(max(-1.0, min(1.0, w))))
    return next(c for lo, hi, low, c in rows if dec_b >= low and lo <= ra_h < hi)


def constellations(src):
    sky = json.loads(src["modern.json"].read_text())
    hyg, designated = hyg_stars(src["hyg.csv"], FAINTEST)
    stars, index, out, missing, figure_stars = [], {}, [], [], {}

    def add(key, ra, dec, v, ci):
        if key not in index:
            index[key] = len(stars)
            stars.append([round(ra, 5), round(dec, 5), round(v, 2),
                          *hex_rgb(bv_rgb(ci) if ci is not None else "#FFFFFF")])
        return index[key]

    def star(hip):
        if hip not in index and hip not in hyg:
            missing.append(hip)
            return None
        return add(hip, *hyg[hip])

    for c in sky["constellations"]:
        abbr = c["id"].split()[-1]
        en = c["common_name"]["native"]
        segs, used = set(), set()
        for line in c["lines"]:
            ids = [star(h) for h in line if isinstance(h, int)]
            for a, b in zip(ids, ids[1:]):
                if a is not None and b is not None and a != b:
                    segs.add((min(a, b), max(a, b)))
                    used.update((a, b))
        figure_stars[abbr] = used
        v = [sum(unit(*stars[i][:2])[k] for i in largest_part(used, segs)) for k in range(3)]
        out.append({"abbr": abbr, "en": en,
                    "zh": ZH[abbr],
                    "lines": sorted([list(s) for s in segs]),
                    "label": [round(t, 3) for t in radec(v)]})
    if missing:
        raise SystemExit(f"figure stars missing from HYG: {sorted(set(missing))}")

    # Every designated star, after all the figure stars so their indices stay put. A star
    # may lie across the border from the constellation its designation names (rho Aql moved
    # into Delphinus in 1992), and the stick figures borrow neighbours' stars (Alpheratz closes
    # the Square of Pegasus): each star belongs to the one constellation whose IAU boundary
    # holds it.
    bounds = iau_boundaries(src["roman87.dat"])
    named = {}      # star -> the constellations its designation or a figure names it for
    for con in out:
        for i in figure_stars[con["abbr"]]:
            named.setdefault(i, set()).add(con["abbr"])
    for hip, s, con in designated:
        named.setdefault(star(hip) if hip is not None else add(("hyg", s), *s), set()).add(con)
    home = [iau_constellation(bounds, *s[:2]) for s in stars]
    inside = {}
    for i, h in enumerate(home):
        inside.setdefault(h, set()).add(i)
    for con in out:
        own_lines, members = figure_stars[con["abbr"]], inside.get(con["abbr"], set())
        con["members"] = sorted(own_lines & members) + sorted(members - own_lines)
    moved = [(stars[i][:3], sorted(named.get(i, ())), h) for i, h in enumerate(home) if named.get(i) != {h}]
    return {"stars": stars, "constellations": out}, moved


def field(seed):
    """Faint background stars, uniform on the sphere thinned by star_density."""
    rng = random.Random(seed)
    out = bytearray()
    n = 0
    while n < FIELD_STARS:
        z, ra = rng.uniform(-1, 1), rng.uniform(0, 360)
        dec = math.degrees(math.asin(z))
        if rng.random() > star_density(*eq2gal(ra, dec)):
            continue
        kind = rng.random()
        if kind < 0.82:                         # the dust: sub-pixel, barely there
            size, alpha, sat = 0.26 + 0.24 * rng.random(), 0.18 + 0.40 * rng.random() ** 2, 1.6
        elif kind < 0.99:                       # crisp pin-points
            size, alpha, sat = 0.40 + 0.60 * rng.random() ** 3, 0.35 + 0.60 * rng.random() ** 1.5, 2.2
        else:                                   # small soft ones
            size, alpha, sat = 0.9 + 0.8 * rng.random() ** 2, 0.22 + 0.32 * rng.random(), 2.4
        r, g, b = hex_rgb(bv_rgb(field_bv(rng), sat))
        out += struct.pack("<ffBBBBf", ra, dec, r, g, b, round(255 * alpha), size)
        n += 1
    return bytes(out)


def milky_way(exr, out):
    """Stars out, diffuse light in: shrink, median-filter, blur, sqrt-encode at 8 bits."""
    magick(exr, "-define", "quantum:format=floating-point", "-resize", MILKY_WAY_SIZE,
           "-statistic", "Median", "7x7", "-blur", "0x2",
           "-channel", "RGB", "-evaluate", "divide", "%[fx:maxima]", "-evaluate", "pow", "0.5", "+channel",
           "-depth", "8", "-define", "png:exclude-chunks=date,time", f"PNG24:{out}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--seed", type=int, default=1977, help="faint background stars seed (default 1977)")
    cache_arg(ap)
    a = ap.parse_args()

    src = fetch(a.cache, SOURCES)
    data, moved = constellations(src)
    print(f"{len(moved)} stars lie in another constellation than the one their name or figure gives (IAU boundaries):")
    for s, was, now in moved:
        print(f"  ra {s[0]:9.4f} dec {s[1]:8.4f} V {s[2]:5.2f}: in {now}, named for {', '.join(was)}")
    OUT.mkdir(parents=True, exist_ok=True)

    text = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    text = re.sub(r'\],\[', "],\n[", text)              # one star / segment list per line: readable diffs
    (OUT / "constellations.json").write_text(text + "\n", encoding="utf-8")
    print(f"wrote constellations.json: {len(data['constellations'])} constellations, "
          f"{len(data['stars'])} stars, {sum(len(c['lines']) for c in data['constellations'])} segments")

    (OUT / "field.bin").write_bytes(field(a.seed))
    print(f"wrote field.bin: {FIELD_STARS} background stars")

    milky_way(src["starmap_2020_4k.exr"], OUT / "milkyway.png")
    print("wrote milkyway.png")

    rsvg(LOGO, 256, 256, OUT / "probe.png")
    print("wrote probe.png")


if __name__ == "__main__":
    main()
