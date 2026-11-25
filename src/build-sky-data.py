#!/usr/bin/env python3
"""Build the sky data the drag-mode wallpaper (awww/voyager-sky) draws.

    src/build-sky-data.py                  # -> awww/sky/{constellations.json,field.bin,milkyway.png,probe.png}
    src/build-sky-data.py --seed 42        # different faint background stars
    src/build-sky-data.py --cache DIR      # where downloads are kept (default ~/.cache/voyager-sky)

Run once; the results are committed. Voyager's own position comes from
src/query-horizons.sh (awww/sky/voyager1.json).

Sources (downloaded once into the cache, never into the repo):
- Stellarium's "modern" sky culture: the 88 IAU constellations, their stick
  figures as Hipparcos numbers (CC BY-SA 4.0), and the Traditional Chinese names
  from Stellarium's zh_TW translation.
- HYG database v4.1 (CC BY-SA 4.0): each figure star's position, V magnitude and
  B-V colour index (from Hipparcos).
- NASA SVS Deep Star Maps 2020 (starmap_2020_4k.exr): the whole sky in ICRF
  plate carree. A median filter removes its stars and leaves the diffuse
  Milky Way (and the Magellanic Clouds), in their true place.

Writes:
- constellations.json  {"stars": [[ra, dec, V, r, g, b]...],
                        "constellations": [{abbr, en, zh, lines: [[i, j]...], label: [ra, dec]}...]}
                        ra/dec in degrees (J2000), r/g/b 0-255 from B-V (voyager_sky.bv_rgb);
                        only stars that are part of a figure are listed.
- field.bin            faint background stars, 16 bytes each, little-endian:
                        f32 ra, f32 dec, u8 r, g, b, alpha, f32 size (px at 1080p).
                        Seeded and fixed on the sphere, denser toward the galactic plane.
- milkyway.png         2048x1024 8-bit RGB, RA 0 at the centre increasing to the left
                        (as the SVS map), Dec +90 at the top; value = sqrt(linear / max).
- probe.png            src/logo.svg at 256 px.

Needs: python3, curl, magick (ImageMagick with OpenEXR), rsvg-convert.
"""

import argparse
import csv
import json
import math
import random
import re
import struct
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import ROOT, LOGO, rsvg  # noqa: E402
from voyager_sky import bv_rgb, eq2gal, field_bv, magick, star_density  # noqa: E402

OUT = ROOT / "awww" / "sky"
STELLARIUM = "https://raw.githubusercontent.com/Stellarium/stellarium/master"
SOURCES = {
    "modern.json": f"{STELLARIUM}/skycultures/modern/index.json",
    "zh_TW.po": f"{STELLARIUM}/po/stellarium-skycultures/zh_TW.po",
    "hyg.csv": "https://raw.githubusercontent.com/astronexus/HYG-Database/main/hyg/CURRENT/hygdata_v41.csv",
    "starmap_2020_4k.exr": "https://svs.gsfc.nasa.gov/vis/a000000/a004800/a004851/starmap_2020_4k.exr",
}

# Taiwan's names, used when Stellarium's zh_TW translation has none.
ZH_FALLBACK = {
    "And": "仙女座", "Ant": "唧筒座", "Aps": "天燕座", "Aqr": "寶瓶座", "Aql": "天鷹座", "Ara": "天壇座",
    "Ari": "白羊座", "Aur": "御夫座", "Boo": "牧夫座", "Cae": "雕具座", "Cam": "鹿豹座", "Cnc": "巨蟹座",
    "CVn": "獵犬座", "CMa": "大犬座", "CMi": "小犬座", "Cap": "摩羯座", "Car": "船底座", "Cas": "仙后座",
    "Cen": "半人馬座", "Cep": "仙王座", "Cet": "鯨魚座", "Cha": "蝘蜓座", "Cir": "圓規座", "Col": "天鴿座",
    "Com": "后髮座", "CrA": "南冕座", "CrB": "北冕座", "Crv": "烏鴉座", "Crt": "巨爵座", "Cru": "南十字座",
    "Cyg": "天鵝座", "Del": "海豚座", "Dor": "劍魚座", "Dra": "天龍座", "Equ": "小馬座", "Eri": "波江座",
    "For": "天爐座", "Gem": "雙子座", "Gru": "天鶴座", "Her": "武仙座", "Hor": "時鐘座", "Hya": "長蛇座",
    "Hyi": "水蛇座", "Ind": "印第安座", "Lac": "蝎虎座", "Leo": "獅子座", "LMi": "小獅座", "Lep": "天兔座",
    "Lib": "天秤座", "Lup": "豺狼座", "Lyn": "天貓座", "Lyr": "天琴座", "Men": "山案座", "Mic": "顯微鏡座",
    "Mon": "麒麟座", "Mus": "蒼蠅座", "Nor": "矩尺座", "Oct": "南極座", "Oph": "蛇夫座", "Ori": "獵戶座",
    "Pav": "孔雀座", "Peg": "飛馬座", "Per": "英仙座", "Phe": "鳳凰座", "Pic": "繪架座", "Psc": "雙魚座",
    "PsA": "南魚座", "Pup": "船尾座", "Pyx": "羅盤座", "Ret": "網罟座", "Sge": "天箭座", "Sgr": "人馬座",
    "Sco": "天蠍座", "Scl": "玉夫座", "Sct": "盾牌座", "Ser": "巨蛇座", "Sex": "六分儀座", "Tau": "金牛座",
    "Tel": "望遠鏡座", "Tri": "三角座", "TrA": "南三角座", "Tuc": "杜鵑座", "UMa": "大熊座", "UMi": "小熊座",
    "Vel": "船帆座", "Vir": "室女座", "Vol": "飛魚座", "Vul": "狐狸座",
}

FIELD_STARS = 60000          # whole sky; a 100-degree view shows ~8000
MILKY_WAY_SIZE = "2048x1024"


def fetch(cache):
    cache.mkdir(parents=True, exist_ok=True)
    for name, url in SOURCES.items():
        path = cache / name
        if not path.exists():
            print(f"downloading {url}")
            part = path.with_suffix(path.suffix + ".part")
            subprocess.run(["curl", "-sSfL", "-o", str(part), url], check=True)
            part.rename(path)
    return {name: cache / name for name in SOURCES}


def zh_names(po_path):
    """msgid -> msgstr for the constellation names in Stellarium's zh_TW.po."""
    names, ctx, msgid = {}, None, None
    for line in po_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("msgctxt "):
            ctx = line[9:-1]
        elif line.startswith("msgid "):
            msgid = line[7:-1]
        elif line.startswith("msgstr ") and msgid:
            value = line[8:-1]
            if value and (ctx is None or "constellation" in ctx):
                names.setdefault(msgid, value)
            ctx, msgid = None, None
    return names


def hyg_stars(path):
    """Hipparcos number -> (ra deg, dec deg, V, B-V or None)."""
    stars = {}
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            if not row["hip"]:
                continue
            hip = int(float(row["hip"]))
            if hip in stars:
                continue
            ci = float(row["ci"]) if row["ci"] else None
            stars[hip] = (float(row["ra"]) * 15.0, float(row["dec"]), float(row["mag"]), ci)
    return stars


def unit(ra, dec):
    ra, dec = math.radians(ra), math.radians(dec)
    return math.cos(dec) * math.cos(ra), math.cos(dec) * math.sin(ra), math.sin(dec)


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


def constellations(src):
    sky = json.loads(src["modern.json"].read_text())
    zh = zh_names(src["zh_TW.po"])
    hyg = hyg_stars(src["hyg.csv"])
    stars, index, out, missing = [], {}, [], []

    def star(hip):
        if hip not in index:
            if hip not in hyg:
                missing.append(hip)
                return None
            ra, dec, v, ci = hyg[hip]
            index[hip] = len(stars)
            stars.append([round(ra, 5), round(dec, 5), round(v, 2),
                          *hex_rgb(bv_rgb(ci) if ci is not None else "#FFFFFF")])
        return index[hip]

    for c in sky["constellations"]:
        abbr = c["id"].split()[-1]
        en = c["common_name"]["native"]
        segs, members = set(), set()
        for line in c["lines"]:
            ids = [star(h) for h in line if isinstance(h, int)]
            for a, b in zip(ids, ids[1:]):
                if a is not None and b is not None and a != b:
                    segs.add((min(a, b), max(a, b)))
                    members.update((a, b))
        v = [sum(unit(*stars[i][:2])[k] for i in largest_part(members, segs)) for k in range(3)]
        out.append({"abbr": abbr, "en": en,
                    "zh": zh.get(en) or zh.get(c["common_name"]["english"]) or ZH_FALLBACK[abbr],
                    "lines": sorted([list(s) for s in segs]),
                    "label": [round(t, 3) for t in radec(v)]})
    if missing:
        raise SystemExit(f"figure stars missing from HYG: {sorted(set(missing))}")
    return {"stars": stars, "constellations": out}


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
    ap.add_argument("--cache", default=str(Path.home() / ".cache" / "voyager-sky"), help="download cache")
    a = ap.parse_args()

    src = fetch(Path(a.cache))
    OUT.mkdir(parents=True, exist_ok=True)

    data = constellations(src)
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
