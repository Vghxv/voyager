"""The downloads behind the sky data, shared by build-sky-data.py and
install-stellarium-figures.py: where they come from (tables/sources), the cache
they are kept in, the HYG star catalogue, and the reader for the tables/ files.
Standard library only; downloads need curl. Not a script.
"""

import csv
import subprocess
from pathlib import Path

CACHE = Path.home() / ".cache" / "voyager-sky"
TABLES = Path(__file__).resolve().parent / "tables"


def table(name):
    """The rows of tables/<name>, each a list of its whitespace-separated columns."""
    return [line.split() for line in (TABLES / name).read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.startswith("#")]


SOURCES = dict(table("sources"))
MODERN = SOURCES["modern.json"].rsplit("/", 1)[0]    # Stellarium's "modern" sky culture


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
