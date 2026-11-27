"""Shared building blocks for the voyager image generators.

Imported by gen-plymouth.py, voyager_sky.py and sky/build-sky-data.py: paths, the palette, rendering,
the --size / --seed options, and the sky maths every sky uses (star colour from
B-V, unit vectors, galactic coordinates, background-star mix and density). Not a script.
"""

import json
import math
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "src" / "art" / "logo.svg"


def read_palette():
    """lib/palette as {name: "#RRGGBB"}."""
    colours = {}
    for line in (ROOT / "lib" / "palette").read_text().splitlines():
        name, _, rest = line.partition(" ")
        if name and not name.startswith("#"):
            colours[name] = rest.split()[0]
    return colours


PALETTE = read_palette()
BG_TOP, BG_BOTTOM = PALETTE["navy"], PALETTE["void"]   # Plymouth Window.SetBackground*Color


def magick(*args):
    try:
        subprocess.run(["magick", *map(str, args)], check=True)
    except FileNotFoundError:
        raise SystemExit("magick not found. Install it: sudo pacman -S imagemagick")


def render(svg, w, h, out):
    """SVG text -> PNG file at w x h."""
    with tempfile.NamedTemporaryFile("w", suffix=".svg") as f:
        f.write(svg)
        f.flush()
        rsvg(f.name, w, h, out)


def rsvg(svg_path, w, h, out):
    """SVG file -> PNG file, with a readable error when rsvg-convert is missing."""
    try:
        subprocess.run(["rsvg-convert", "-w", str(w), "-h", str(h), str(svg_path), "-o", str(out)],
                       check=True)
    except FileNotFoundError:
        raise SystemExit("rsvg-convert not found. Install it: sudo pacman -S librsvg")


def sky_args(ap):
    """Add the --size and --seed options every sky generator takes."""
    ap.add_argument("--size", default="1920x1080",
                    help="WxH, or 'auto' for the largest connected monitor (default 1920x1080)")
    ap.add_argument("--seed", type=int, default=1977, help="faint stars and dust texture seed (default 1977)")


def parse_size(size):
    """'WxH', or 'auto' for the largest connected monitor (hyprctl), rotation included."""
    if size.lower() != "auto":
        try:
            w, h = (int(v) for v in size.lower().split("x"))
        except ValueError:
            raise SystemExit(f"--size {size!r}: expected WxH (e.g. 2560x1440) or auto")
        return w, h
    try:
        mons = json.loads(subprocess.run(["hyprctl", "monitors", "-j"], capture_output=True,
                                         text=True, check=True).stdout)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as e:
        raise SystemExit(f"--size auto needs a running Hyprland (hyprctl monitors -j): {e}")
    if not mons:
        raise SystemExit("--size auto: hyprctl reports no monitors")
    # transforms 1, 3, 5, 7 rotate by 90 degrees and swap width and height
    sizes = [(m["height"], m["width"]) if m.get("transform", 0) % 2 else (m["width"], m["height"])
             for m in mons]
    return max(sizes, key=lambda wh: wh[0] * wh[1])


# --- the sky: star colours and galactic coordinates -------------------------------

NGP = (192.85948, 27.12825, 122.93192)     # galactic north pole RA, Dec; l of the NCP

# Spectral mix of the faint background, as the eye sees a dark sky: (weight, B-V range).
STAR_CLASSES = [(0.30, (-0.30, 0.00)),        # blue: O, B
                (0.34, (0.00, 0.35)),         # white: A, early F
                (0.26, (0.50, 0.95)),         # yellow: G
                (0.10, (1.00, 1.50))]         # orange: K


def bv_rgb(bv, sat=1.0):
    """B-V colour index -> sRGB hex of a blackbody at that star's temperature.

    sat > 1 pushes the tint away from white, so it still reads on a sub-pixel dot."""
    bv = max(-0.3, min(1.8, bv))
    t = 4600 * (1 / (0.92 * bv + 1.7) + 1 / (0.92 * bv + 0.62)) / 100   # Ballesteros 2012
    if t <= 66:
        r, g = 255, 99.4708025861 * math.log(t) - 161.1195681661
        b = 0 if t <= 19 else 138.5177312231 * math.log(t - 10) - 305.0447927307
    else:
        r = 329.698727446 * (t - 60) ** -0.1332047592
        g = 288.1221695283 * (t - 60) ** -0.0755148492
        b = 255
    return "#" + "".join(f"{max(0, min(255, round(255 - (255 - c) * sat))):02X}" for c in (r, g, b))


def field_bv(rng):
    """A random B-V index for a background star, following STAR_CLASSES."""
    pick, acc = rng.random(), 0.0
    for weight, (lo, hi) in STAR_CLASSES:
        acc += weight
        if pick <= acc:
            return rng.uniform(lo, hi)
    return 1.2


def unit(ra, dec):
    """RA, Dec (degrees) -> unit vector (x toward RA 0, z toward the north pole)."""
    ra, dec = math.radians(ra), math.radians(dec)
    return math.cos(dec) * math.cos(ra), math.cos(dec) * math.sin(ra), math.sin(dec)


def eq2gal(ra, dec):
    """Equatorial (RA, Dec) -> galactic (l, b), degrees."""
    an, dn, ln = (math.radians(t) for t in NGP)
    a, d = math.radians(ra), math.radians(dec)
    b = math.asin(math.sin(d) * math.sin(dn) + math.cos(d) * math.cos(dn) * math.cos(a - an))
    l = ln - math.atan2(math.cos(d) * math.sin(a - an),
                        math.sin(d) * math.cos(dn) - math.cos(d) * math.sin(dn) * math.cos(a - an))
    return math.degrees(l) % 360, math.degrees(b)


def gal2eq(l, b):
    """Galactic (l, b) -> equatorial (RA, Dec), degrees."""
    an, dn, ln = (math.radians(t) for t in NGP)
    l, b = math.radians(l), math.radians(b)
    d = math.asin(math.sin(dn) * math.sin(b) + math.cos(dn) * math.cos(b) * math.cos(ln - l))
    a = an + math.atan2(math.cos(b) * math.sin(ln - l),
                        math.cos(dn) * math.sin(b) - math.sin(dn) * math.cos(b) * math.cos(ln - l))
    return math.degrees(a) % 360, math.degrees(d)


def star_density(l, b):
    """Relative density of faint background stars: more toward the galactic plane."""
    return 0.35 + 0.65 * min(1.0, math.exp(-(b - 20) / 20))
