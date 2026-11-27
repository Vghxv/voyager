"""The downloads behind the sky data, shared by build-sky-data.py and
install-stellarium-figures.py: where they come from, the cache they are kept in,
and the HYG star catalogue. Standard library only. Not a script.
"""

import csv
import subprocess
from pathlib import Path

CACHE = Path.home() / ".cache" / "voyager-sky"
STELLARIUM = "https://raw.githubusercontent.com/Stellarium/stellarium/master"
MODERN = f"{STELLARIUM}/skycultures/modern"      # Stellarium's "modern" sky culture
SOURCES = {
    "modern.json": f"{MODERN}/index.json",
    "hyg.csv": "https://raw.githubusercontent.com/astronexus/HYG-Database/main/hyg/CURRENT/hygdata_v41.csv",
    "starmap_2020_4k.exr": "https://svs.gsfc.nasa.gov/vis/a000000/a004800/a004851/starmap_2020_4k.exr",
    # Roman (1987), CDS VI/42: the IAU boundaries (Delporte 1930) as B1875 boxes. Astropy's copy,
    # as the CDS host is not always reachable.
    "roman87.dat": "https://raw.githubusercontent.com/astropy/astropy/main/"
                   "astropy/coordinates/data/constellation_data_roman87.dat",
}


def cache_arg(ap):
    """Add --cache (where downloads are kept) to an argparse parser."""
    ap.add_argument("--cache", type=Path, default=CACHE, help=f"download cache (default {CACHE})")


def fetch(cache, names, sources=SOURCES):
    """name -> local path for each of `names`, downloading the ones not yet in `cache`."""
    cache.mkdir(parents=True, exist_ok=True)
    paths = {}
    for name in names:
        path = paths[name] = cache / name
        if not path.exists():
            print(f"downloading {sources[name]}")
            path.parent.mkdir(parents=True, exist_ok=True)
            part = path.with_suffix(path.suffix + ".part")
            subprocess.run(["curl", "-sSfL", "-o", str(part), sources[name]], check=True)
            part.rename(path)
    return paths


def hyg_stars(path, faintest=None):
    """Hipparcos number -> (ra deg, dec deg, V, B-V or None), and the designated
    (Bayer/Flamsteed) stars brighter than `faintest` as (hip or None, star, the
    constellation their designation names); none without `faintest`."""
    stars, designated = {}, []
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            ci = float(row["ci"]) if row["ci"] else None
            star = (float(row["ra"]) * 15.0, float(row["dec"]), float(row["mag"]), ci)
            hip = int(float(row["hip"])) if row["hip"] else None
            if hip is not None:
                stars.setdefault(hip, star)
            if faintest is not None and row["con"] and (row["bayer"] or row["flam"]) and star[2] < faintest:
                designated.append((hip, star, row["con"]))
    return stars, designated
