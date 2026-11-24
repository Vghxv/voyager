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
twinkle; everything else is rendered once and stays still.

Needs: python3, rsvg-convert (librsvg), magick (ImageMagick). ffmpeg makes the
WebP about 4x smaller and is used when installed.
"""

import argparse
import math
import random
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True          # no src/__pycache__
from voyager_art import ROOT, LOGO, BG_TOP, BG_BOTTOM, render, parse_size  # noqa: E402

# (name, x, y, V magnitude, B-V, RA, Dec). x/y are pixel centroids measured on the
# 800x615 reference photo; the rest are catalogue values (RA/Dec in degrees, J2000).
CONSTELLATIONS = [
    # Ursa Major: the Big Dipper
    ("Alkaid",  100.9, 106.4, 1.86, -0.19, 206.885, 49.313),
    ("Mizar",   167.3, 194.4, 2.23,  0.06, 200.981, 54.925),
    ("Alioth",  166.1, 263.3, 1.77, -0.02, 193.507, 55.960),
    ("Megrez",  172.0, 348.9, 3.31,  0.08, 183.857, 57.033),
    ("Phecda",  116.7, 397.1, 2.44,  0.04, 178.457, 53.695),
    ("Merak",   174.4, 509.3, 2.37,  0.03, 165.460, 56.383),
    ("Dubhe",   257.7, 484.2, 1.79,  1.07, 165.932, 61.751),
    # Ursa Minor: the Little Dipper
    ("Polaris", 700.9, 380.1, 1.98,  0.60,  37.954, 89.264),
    ("Yildun",  680.9, 319.1, 4.35,  0.03, 263.054, 86.586),
    ("Eps UMi", 642.9, 253.9, 4.21,  0.89, 251.493, 82.037),
    ("Zet UMi", 572.0, 217.2, 4.32,  0.04, 236.015, 77.794),
    ("Eta UMi", 579.9, 174.3, 4.95,  0.37, 244.376, 75.755),
    ("Pherkad", 492.4, 162.4, 3.00,  0.06, 230.182, 71.834),
    ("Kochab",  495.6, 211.7, 2.08,  1.47, 222.676, 74.156),
    # fainter field stars that stand out in the photo (not used for the fit)
    ("",        335.5, 181.1, 5.0,   0.0,  None, None),
    ("",        374.8, 333.5, 4.9,   0.1,  None, None),
    ("",        366.0, 417.9, 5.1,   0.9,  None, None),
]
URSA_MAJOR = {"Alkaid", "Mizar", "Alioth", "Megrez", "Phecda", "Merak", "Dubhe"}
UMA_HALO = 0.72                         # the Big Dipper's bright stars get a tighter halo
REF_BOX = (100.9, 106.4, 700.9, 509.3)  # bounding box of the stars above, in photo pixels
SPAN = 0.42                             # constellation width, as a fraction of a 16:9 screen width
FIELD_TWINKLERS = 5                     # background stars that twinkle, besides Polaris

# Deep-sky objects: (name, RA, Dec, major and minor axis in arcmin, position angle
# in degrees from north through east, colour, opacity). Catalogue values (J2000).
GALAXIES = [
    ("M81",      148.888, 69.065, 26.9, 14.1, 157, "#F3E3C6", 0.70),  # Bode's Galaxy
    ("M82",      148.968, 69.680, 11.2,  4.3,  65, "#EDE0D4", 0.70),  # Cigar Galaxy
    ("M101",     210.802, 54.349, 28.8, 26.9,   0, "#C9D3F2", 0.30),  # Pinwheel, faint face-on
    ("M108",     167.879, 55.674,  8.7,  2.2,  79, "#DADDEB", 0.50),
    ("M109",     179.400, 53.375,  7.6,  4.7,  23, "#E2DDD6", 0.45),
    ("NGC 6217", 248.163, 78.198,  3.0,  2.5, 162, "#D6DAEC", 0.40),
]
OWL = (168.699, 55.019, 3.4, "#7FE3D6")       # M97, the Owl Nebula: RA, Dec, size', colour

# Integrated Flux Nebula. Its brightness follows galactic latitude b (a general
# cirrus that thickens toward the plane) plus two known clouds, in galactic (l, b):
IFN_CIRRUS_B = (22.0, 9.0)                    # full strength at b <= 22, e-folds every 9 deg
POLARIS_FLARE = (123.0, 29.0, 4.0, 6.0, 0.9)  # l, b, sigma_l, sigma_b (deg), strength
VOLCANO = (142.1, 40.9, 5.0, 0.6)             # the dust around M81/M82: l, b, sigma, strength
IFN_LAYERS = [  # (colour, opacity, noise seed offset): screened onto the sky
    ("#A39383", 0.34, 0),                     # grey-brown dust in scattered starlight
    ("#7E93B8", 0.16, 1),                     # bluer scattering
    ("#B0706A", 0.09, 2),                     # faint red luminescence
]


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


def mag_scale(m):
    """Linear size factor for magnitude m: 1 at mag 1.8, smaller for fainter stars."""
    return 10 ** (-0.2 * (m - 1.8))


class Sky:
    """Sky <-> screen. Fits the catalogue positions of the dipper stars to where they
    sit in the photo (gnomonic projection + rotation/scale/shift; RMS error < 1 photo
    px), then scales the photo's star box to SPAN of the screen width, centred."""

    NGP = (192.85948, 27.12825, 122.93192)     # galactic north pole RA, Dec; l of the NCP

    def __init__(self, w, h):
        stars = [c for c in CONSTELLATIONS if c[5] is not None]
        v = [(math.cos(math.radians(d)) * math.cos(math.radians(r)),
              math.cos(math.radians(d)) * math.sin(math.radians(r)), math.sin(math.radians(d)))
             for *_, r, d in stars]
        c = [sum(p[i] for p in v) for i in range(3)]
        self.ra0 = math.atan2(c[1], c[0])
        self.de0 = math.asin(c[2] / math.sqrt(sum(t * t for t in c)))
        z = [self._gnomonic(r, d) for *_, r, d in stars]
        p = [complex(x, y) for _, x, y, *_ in stars]
        zm, pm = sum(z) / len(z), sum(p) / len(p)
        self.a = (sum((pi - pm) * (zi - zm).conjugate() for zi, pi in zip(z, p))
                  / sum(abs(zi - zm) ** 2 for zi in z))
        self.b = pm - self.a * zm
        x0, y0, x1, y1 = REF_BOX
        self.s = SPAN * min(w, h * 16 / 9) / (x1 - x0)
        self.pc = complex((x0 + x1) / 2, (y0 + y1) / 2)
        self.sc = complex(w / 2, h / 2)
        self.px_per_deg = abs(self.a) * self.s * math.pi / 180

    def _gnomonic(self, ra, dec):
        a, d = math.radians(ra) - self.ra0, math.radians(dec)
        cc = math.sin(self.de0) * math.sin(d) + math.cos(self.de0) * math.cos(d) * math.cos(a)
        return complex(math.cos(d) * math.sin(a) / cc,
                       (math.cos(self.de0) * math.sin(d) - math.sin(self.de0) * math.cos(d) * math.cos(a)) / cc)

    def photo(self, x, y):
        q = (complex(x, y) - self.pc) * self.s + self.sc
        return q.real, q.imag

    def screen(self, ra, dec):
        p = self.a * self._gnomonic(ra, dec) + self.b
        return self.photo(p.real, p.imag)

    def radec(self, x, y):
        z = ((complex(x, y) - self.sc) / self.s + self.pc - self.b) / self.a
        rho = abs(z)
        if rho == 0:
            return math.degrees(self.ra0), math.degrees(self.de0)
        c = math.atan(rho)
        dec = math.asin(math.cos(c) * math.sin(self.de0) + z.imag * math.sin(c) * math.cos(self.de0) / rho)
        ra = self.ra0 + math.atan2(z.real * math.sin(c), rho * math.cos(self.de0) * math.cos(c)
                                   - z.imag * math.sin(self.de0) * math.sin(c))
        return math.degrees(ra) % 360, math.degrees(dec)

    def galactic(self, ra, dec):
        an, dn, ln = (math.radians(t) for t in self.NGP)
        a, d = math.radians(ra), math.radians(dec)
        b = math.asin(math.sin(d) * math.sin(dn) + math.cos(d) * math.cos(dn) * math.cos(a - an))
        l = ln - math.atan2(math.cos(d) * math.sin(a - an),
                            math.sin(d) * math.cos(dn) - math.cos(d) * math.sin(dn) * math.cos(a - an))
        return math.degrees(l) % 360, math.degrees(b)

    def lb(self, x, y):
        return self.galactic(*self.radec(x, y))

    def axes(self, ra, dec):
        """Unit screen vectors pointing north and east at (ra, dec)."""
        x, y = self.screen(ra, dec)
        nx, ny = self.screen(ra, dec + 0.05)
        ex, ey = self.screen(ra + 0.05 / math.cos(math.radians(dec)), dec)
        n, e = math.hypot(nx - x, ny - y), math.hypot(ex - x, ey - y)
        return ((nx - x) / n, (ny - y) / n), ((ex - x) / e, (ey - y) / e)


def ifn_strength(l, b):
    """0..1: how much Integrated Flux Nebula a long exposure shows at galactic (l, b)."""
    b0, fold = IFN_CIRRUS_B
    cirrus = min(1.0, math.exp(-(b - b0) / fold))
    fl, fb, sl, sb, k = POLARIS_FLARE
    dl = ((l - fl + 180) % 360 - 180) * math.cos(math.radians(b))
    flare = k * math.exp(-(dl / sl) ** 2 - ((b - fb) / sb) ** 2)
    vl, vb, vs, vk = VOLCANO
    dl = ((l - vl + 180) % 360 - 180) * math.cos(math.radians(b))
    volcano = vk * math.exp(-(dl ** 2 + (b - vb) ** 2) / vs ** 2)
    return min(1.0, 0.55 * cirrus + flare + volcano)


def star_density(l, b):
    """Relative density of faint background stars: more toward the galactic plane."""
    return 0.35 + 0.65 * min(1.0, math.exp(-(b - 20) / 20))


def twinkle(cycles, phase, frame, frames, depth):
    """Brightness multiplier; whole cycles per loop so the animation loops seamlessly."""
    if frames == 1:
        return 1.0
    return 1 - depth + depth * math.cos(2 * math.pi * cycles * frame / frames + phase)


# --- background stars ----------------------------------------------------------

# Spectral mix of the background, as the eye sees a dark sky: (weight, B-V range).
STAR_CLASSES = [(0.30, (-0.30, 0.00)),   # blue: O, B
                (0.34, (0.00, 0.35)),    # white: A, early F
                (0.26, (0.50, 0.95)),    # yellow: G
                (0.10, (1.00, 1.50))]    # orange: K


def field_colour(rng, sat):
    pick, acc = rng.random(), 0.0
    for weight, (lo, hi) in STAR_CLASSES:
        acc += weight
        if pick <= acc:
            return bv_rgb(rng.uniform(lo, hi), sat)
    return bv_rgb(1.2, sat)


def field(rng, w, h, sky):
    """Background stars: (x, y, r, opacity, colour, glow).

    Counts follow a steep luminosity function, as a long exposure records them:
    thousands of faint dots for every bright one, thickest toward the galactic plane.
    """
    u = w / 1920
    area = w * h / (1920 * 1080)
    out = []

    def spot():
        while True:
            x, y = rng.uniform(0, w), rng.uniform(0, h)
            if rng.random() < star_density(*sky.lb(x, y)):
                return x, y

    for _ in range(int(16000 * area)):            # the faint dust: sub-pixel, barely there
        x, y = spot()
        out.append((x, y, (0.26 + 0.24 * rng.random()) * u, 0.18 + 0.40 * rng.random() ** 2,
                    field_colour(rng, 1.6), False))
    for _ in range(int(3200 * area)):             # crisp pin-points
        x, y = spot()
        k = rng.random() ** 3
        out.append((x, y, (0.40 + 0.60 * k) * u, 0.35 + 0.60 * rng.random() ** 1.5,
                    field_colour(rng, 2.2), False))
    for _ in range(int(160 * area)):              # small soft ones, all dimmer than mag 5
        x, y = spot()
        k = rng.random() ** 2
        out.append((x, y, (0.9 + 1.1 * k) * u, 0.22 + 0.32 * rng.random(),
                    field_colour(rng, 2.4), True))
    return out


def field_star(j, x, y, r, a, col, glow):
    """SVG for one background star, plus its gradient def if it has a glow."""
    if not glow:
        return "", f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r:.2f}" fill="{col}" opacity="{a:.2f}"/>'
    return (f'<radialGradient id="f{j}"><stop offset="0" stop-color="#FFFFFF"/>'
            f'<stop offset="0.3" stop-color="{col}" stop-opacity="0.6"/>'
            f'<stop offset="1" stop-color="{col}" stop-opacity="0"/></radialGradient>',
            f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r * 2.2:.2f}" fill="url(#f{j})" opacity="{a:.2f}"/>')


# --- sky and nebula (raster) ---------------------------------------------------
# Drawn with ImageMagick at 16 bits per channel and dithered down to 8, so the dark
# gradient and the faint clouds have no banding and no blocks. (librsvg's
# feTurbulence also leaves square holes at large sizes, so no SVG filters here.)

def magick(*args):
    try:
        subprocess.run(["magick", *map(str, args)], check=True)
    except FileNotFoundError:
        raise SystemExit("magick not found. Install it: sudo pacman -S imagemagick")


def fbm(w, h, seed, out, cells=6, octaves=8, lo=44, hi=96, aspect=1.0, angle=0.0):
    """Fractal cloud noise: random grids upscaled smoothly and summed, finest last.

    aspect > 1 stretches the texture into streaks, which are then turned by angle."""
    side = math.ceil(math.hypot(w, h))
    args = ["-seed", seed]
    weights = []
    for k in range(octaves):
        cw, ch = cells * 2 ** k, max(2, round(cells * 2 ** k * aspect))
        args += ["(", "-size", f"{cw}x{ch}", "xc:gray", "+noise", "Random", "-colorspace", "gray",
                 "-filter", "Cubic", "-resize", f"{side}x{side}!", ")"]
        weights.append(f"{0.68 ** k:.4f},1")
    magick(*args, "-poly", " ".join(weights), "-auto-level", "-level", f"{lo}%,{hi}%",
           "-background", "black", "-rotate", f"{angle:.1f}", "+repage",
           "-gravity", "center", "-extent", f"{w}x{h}", out)


def ifn_mask(w, h, sky, out, step=8):
    """Grey map of ifn_strength over the screen, computed coarsely, smoothed when used."""
    gw, gh = math.ceil(w / step), math.ceil(h / step)
    px = bytes(round(255 * ifn_strength(*sky.lb((i + 0.5) * step, (j + 0.5) * step)))
               for j in range(gh) for i in range(gw))
    Path(out).write_bytes(f"P5 {gw} {gh} 255\n".encode() + px)


def backdrop(w, h, seed, sky, out, tmp):
    """Gradient sky with the Integrated Flux Nebula screened on, dithered to 8 bits.

    The dust's fractal texture is stretched 2:1 along the direction of galactic
    latitude (as the Polaris Flare's filaments run) and shown where ifn_strength is."""
    mask = f"{tmp}/ifn.pgm"
    ifn_mask(w, h, sky, mask)
    fx, fy = sky.screen(*gal2eq(POLARIS_FLARE[0], POLARIS_FLARE[1]))
    gx, gy = sky.screen(*gal2eq(POLARIS_FLARE[0], POLARIS_FLARE[1] + 5))
    angle = math.degrees(math.atan2(gy - fy, gx - fx))
    args = ["-size", f"{w}x{h}", f"gradient:{BG_TOP}-{BG_BOTTOM}"]
    for col, op, k in IFN_LAYERS:
        noise = f"{tmp}/ifn{k}.png"
        fbm(w, h, seed + k, noise, cells=4 + k, aspect=2.0, angle=angle, lo=40, hi=97)
        args += ["(", "-size", f"{w}x{h}", f"xc:{col}",
                 "(", noise, "(", mask, "-filter", "Cubic", "-resize", f"{w}x{h}!", ")",
                 "-compose", "Multiply", "-composite", "-colorspace", "sRGB", ")",
                 "-compose", "Multiply", "-composite", "-evaluate", "multiply", op, ")",
                 "-compose", "Screen", "-composite"]
    magick(*args, "-attenuate", DITHER, "+noise", "Uniform", "-depth", 8, out)


def gal2eq(l, b):
    """Galactic (l, b) -> equatorial (RA, Dec), degrees."""
    an, dn, ln = (math.radians(t) for t in Sky.NGP)
    l, b = math.radians(l), math.radians(b)
    d = math.asin(math.sin(dn) * math.sin(b) + math.cos(dn) * math.cos(b) * math.cos(ln - l))
    a = an + math.atan2(math.cos(b) * math.sin(ln - l),
                        math.cos(dn) * math.sin(b) - math.sin(dn) * math.cos(b) * math.cos(ln - l))
    return math.degrees(a) % 360, math.degrees(d)


DITHER = 0.35   # +noise Uniform strength: about one 8-bit step, enough to break up banding


# --- constellations and probe --------------------------------------------------

def star_svg(i, x, y, core, glow, col, op=1.0):
    """One star: a tinted halo plus a crisp core, and its gradient def."""
    d = (f'<radialGradient id="g{i}"><stop offset="0" stop-color="#FFFFFF" stop-opacity="0.95"/>'
         f'<stop offset="0.18" stop-color="{col}" stop-opacity="0.75"/>'
         f'<stop offset="0.45" stop-color="{col}" stop-opacity="0.18"/>'
         f'<stop offset="1" stop-color="{col}" stop-opacity="0"/></radialGradient>')
    s = (f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{glow:.2f}" fill="url(#g{i})" opacity="{op:.3f}"/>'
         f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{core:.2f}" fill="{col}" opacity="{min(1, 0.35 + op * 0.65):.3f}"/>')
    return d, s


def constellation(w, h, sky, name_filter, frame=0, frames=1):
    """The dipper stars whose name passes name_filter; Polaris twinkles and glints."""
    u = w / 1920
    defs, out = [], []
    for i, (name, x, y, m, bv, *_) in enumerate(CONSTELLATIONS):
        if not name_filter(name):
            continue
        sx, sy = sky.photo(x, y)
        col = bv_rgb(bv)
        f = mag_scale(m)
        op = twinkle(1, 0.0, frame, frames, 0.45) if name == "Polaris" else 1.0
        halo = UMA_HALO if name in URSA_MAJOR else 1.0
        d, s = star_svg(i, sx, sy, (0.7 + 1.9 * f) * u, (3.5 + 15 * f) * halo * u, col, op)
        defs.append(d)
        out.append(s)
        if name == "Polaris":                     # a thin four-point glint that breathes
            arm = (10 + 7 * op) * u
            out.append(f'<g stroke="{col}" stroke-width="{0.45 * u:.2f}" stroke-linecap="round" '
                       f'opacity="{0.15 + 0.35 * op:.3f}">'
                       f'<line x1="{sx - arm:.2f}" y1="{sy:.2f}" x2="{sx + arm:.2f}" y2="{sy:.2f}"/>'
                       f'<line x1="{sx:.2f}" y1="{sy - arm:.2f}" x2="{sx:.2f}" y2="{sy + arm:.2f}"/></g>')
    return "\n".join(defs), "\n".join(out)


def deep_sky(sky):
    """The galaxies and the Owl Nebula at their true position, size and angle."""
    k = sky.px_per_deg / 60                       # px per arcminute
    defs, out = [], []
    for i, (name, ra, dec, major, minor, pa, col, op) in enumerate(GALAXIES):
        x, y = sky.screen(ra, dec)
        (nx, ny), (ex, ey) = sky.axes(ra, dec)
        t = math.radians(pa)
        rot = math.degrees(math.atan2(math.cos(t) * ny + math.sin(t) * ey, math.cos(t) * nx + math.sin(t) * ex))
        rx, ry = max(0.7, major * k / 2), max(0.45, minor * k / 2)
        defs.append(f'<radialGradient id="gx{i}"><stop offset="0" stop-color="#FFFFFF" stop-opacity="0.95"/>'
                    f'<stop offset="0.2" stop-color="{col}" stop-opacity="0.7"/>'
                    f'<stop offset="0.6" stop-color="{col}" stop-opacity="0.22"/>'
                    f'<stop offset="1" stop-color="{col}" stop-opacity="0"/></radialGradient>')
        g = (f'<g transform="translate({x:.2f} {y:.2f}) rotate({rot:.1f})" opacity="{op}">'
             f'<ellipse rx="{rx:.2f}" ry="{ry:.2f}" fill="url(#gx{i})"/>')
        if name == "M82":                         # its red hydrogen plume, across the disc
            g += (f'<ellipse rx="{rx * 0.22:.2f}" ry="{ry * 1.9:.2f}" fill="#FF6A5A" opacity="0.22"/>')
        out.append(g + "</g>")
    ra, dec, size, col = OWL
    x, y = sky.screen(ra, dec)
    r = max(1.3, size * k)
    defs.append(f'<radialGradient id="owl"><stop offset="0" stop-color="{col}" stop-opacity="0.9"/>'
                f'<stop offset="0.55" stop-color="{col}" stop-opacity="0.55"/>'
                f'<stop offset="1" stop-color="{col}" stop-opacity="0"/></radialGradient>')
    out.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r:.2f}" fill="url(#owl)"/>')
    return "".join(defs), "\n".join(out)


def probe(w, h):
    """The probe, tiny, in the lower right."""
    size = 0.016 * w
    px, py = 0.80 * w, 0.76 * h                  # top-left of the logo box
    body = LOGO.read_text()
    return re.sub(r"<svg[^>]*>", f'<svg x="{px:.2f}" y="{py:.2f}" width="{size:.2f}" '
                  f'height="{size:.2f}" viewBox="0 0 400 400">', body, count=1)


# --- assembly ------------------------------------------------------------------

class Scene:
    """Splits the wallpaper into a still layer and a small twinkling layer."""

    def __init__(self, w, h, seed):
        self.w, self.h, self.seed = w, h, seed
        self.sky = Sky(w, h)
        rng = random.Random(seed)
        self.stars = field(rng, w, h, self.sky)
        soft = [j for j, s in enumerate(self.stars) if s[5] and s[1] < 0.95 * h]
        self.twinklers = {j: (rng.choice((1, 2)), rng.uniform(0, 2 * math.pi))
                          for j in rng.sample(soft, min(FIELD_TWINKLERS, len(soft)))}

    def svg(self, body, defs=""):
        w, h = self.w, self.h
        return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">\n'
                f'<defs>{defs}</defs>\n{body}\n</svg>\n')

    def still(self, with_twinklers):
        """Transparent layer of every star that never changes, and the probe;
        with_twinklers adds the twinkling stars at rest (for the still PNG)."""
        f_defs, f = [], []
        for j, s in enumerate(self.stars):
            if j in self.twinklers and not with_twinklers:
                continue
            d, c = field_star(j, *s)
            f_defs.append(d)
            f.append(c)
        keep = (lambda name: True) if with_twinklers else (lambda name: name != "Polaris")
        c_defs, c = constellation(self.w, self.h, self.sky, keep)
        d_defs, d = deep_sky(self.sky)
        return self.svg("\n".join(f) + f"\n{d}\n{c}\n{probe(self.w, self.h)}",
                        "".join(f_defs) + d_defs + c_defs)

    def twinkling(self, frame, frames):
        """Transparent layer: Polaris and the few twinkling background stars."""
        defs, out = [], []
        for j, (cycles, phase) in self.twinklers.items():
            x, y, r, a, col, glow = self.stars[j]
            a = min(1, a * 1.8 * twinkle(cycles, phase, frame, frames, 0.5))
            d, c = field_star(j, x, y, r, a, col, glow)
            defs.append(d)
            out.append(c)
        c_defs, c = constellation(self.w, self.h, self.sky, lambda name: name == "Polaris", frame, frames)
        return self.svg("\n".join(out) + "\n" + c, "".join(defs) + c_defs)


def encode_webp(pngs, pattern, fps, out):
    """Lossless animated WebP (lossy turns the dark gradient into visible blocks).

    ffmpeg's encoder stores each frame as just the pixels that changed, which here
    is a few twinkling stars: ~1.5 MB. ImageMagick's re-stores the whole changed
    rectangle every frame (~4x larger, ~20x slower), so it is only the fallback.
    """
    try:
        subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-framerate", str(fps),
                        "-i", pattern, "-c:v", "libwebp_anim", "-lossless", "1",
                        "-compression_level", "6", "-loop", "0", "-pix_fmt", "bgra", out], check=True)
    except FileNotFoundError:
        print("ffmpeg not found, encoding with ImageMagick (bigger file). "
              "For the small one: sudo pacman -S ffmpeg", file=sys.stderr)
        magick("-delay", f"{100 / fps:.2f}", *pngs, "-loop", 0,     # delay is in 1/100 s
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
    ap.add_argument("--svg", help="also keep the static SVG here")
    a = ap.parse_args()

    w, h = parse_size(a.size)
    scene = Scene(w, h, a.seed)
    static = scene.still(with_twinklers=True)
    if a.svg:
        Path(a.svg).write_text(static)
    with tempfile.TemporaryDirectory() as tmp:
        sky = f"{tmp}/sky.png"
        backdrop(w, h, a.seed, scene.sky, sky, tmp)
        layer = f"{tmp}/still.png"
        render(static, w, h, layer)
        magick(sky, layer, "-composite", a.out)
        print(f"wrote {a.out}")

        if a.frames > 0:
            base = f"{tmp}/base.png"
            render(scene.still(with_twinklers=False), w, h, layer)
            magick(sky, layer, "-composite", base)
            pngs = []
            for i in range(a.frames):
                tw, png = f"{tmp}/t{i:03d}.png", f"{tmp}/f{i:03d}.png"
                render(scene.twinkling(i, a.frames), w, h, tw)
                magick(base, tw, "-composite", png)
                pngs.append(png)
            encode_webp(pngs, f"{tmp}/f%03d.png", a.fps, a.anim)
            print(f"wrote {a.anim} ({a.frames} frames @ {a.fps} fps, lossless)")
    print(f"{w}x{h}, seed {a.seed}")


if __name__ == "__main__":
    main()
