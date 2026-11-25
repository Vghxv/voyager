"""The boot screen's night sky: Ursa Major and Ursa Minor in a true-to-sky long exposure.

Used by gen-plymouth.py. Not a script.

The dipper stars are laid out as in a wide-field photo of the two (the Big Dipper's
handle up on the left, its pointers aiming right at Polaris) and drawn by magnitude
and true colour (B-V index -> temperature -> blackbody RGB). Their catalogue
positions are fitted to the photo (a gnomonic projection, < 1 px error), and the
same fit places everything else where a long exposure of this field shows it:
- the Integrated Flux Nebula: faint grey-brown dust lit by the whole Milky Way,
  thickening toward lower galactic latitude (the right of the screen), with the
  Polaris Flare beside Polaris and the cloud around M81/M82;
- the Owl Nebula (M97), a teal planetary nebula next to Merak;
- the galaxies M81, M82, M101, M108, M109 and NGC 6217, at true size and angle;
- thousands of faint blue, white and yellow stars, denser toward the galactic plane.

How it is built: the sky and dust are a 16-bit ImageMagick raster dithered to 8
bits (no banding); every star and galaxy is one SVG layer rendered once
(render_base); Polaris, a few faint stars and the Doppler-shifted stars are
separate transparent layers (Scene.*_layer).

Needs: rsvg-convert (librsvg), magick (ImageMagick).
"""

import math
import os
import random
import tempfile
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from pathlib import Path

from voyager_art import (BG_TOP, BG_BOTTOM, NGP, bv_rgb, eq2gal, field_bv, magick,
                         render, star_density)

# --- catalogue ---------------------------------------------------------------------

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

# --- look ------------------------------------------------------------------------

REF_BOX = (100.9, 106.4, 700.9, 509.3)  # bounding box of the dipper stars, in photo pixels
SPAN = 0.42                             # their width, as a fraction of a 16:9 screen width
UMA_HALO = 0.72                         # the Big Dipper's bright stars get a tighter halo
FIELD_TWINKLERS = 5                     # background stars that twinkle, besides Polaris
DITHER = 0.35                           # +noise Uniform strength: about one 8-bit step
# Doppler layers (Plymouth fades them in as the probe speeds up or backs off): every
# constellation star and this share of the background stars, their B-V moved by
# BLUE_SHIFT / RED_SHIFT and their tint pushed SHIFT_SAT times further from white,
# so the shift reads on tiny dots.
SHIFT_FRACTION, BLUE_SHIFT, RED_SHIFT, SHIFT_SAT = 0.9, -0.9, 1.1, 1.4


# --- colour ------------------------------------------------------------------------

def mag_scale(m):
    """Linear size factor for magnitude m: 1 at mag 1.8, smaller for fainter stars."""
    return 10 ** (-0.2 * (m - 1.8))


def twinkle(cycles, phase, frame, frames, depth):
    """Brightness multiplier, 1 at rest; whole cycles per loop so it loops seamlessly."""
    if frames <= 1:
        return 1.0
    return 1 - depth + depth * math.cos(2 * math.pi * cycles * frame / frames + phase)


# --- sky geometry ------------------------------------------------------------------

class Sky:
    """Sky <-> screen. Fits the catalogue positions of the dipper stars to where they
    sit in the photo (gnomonic projection + rotation/scale/shift; RMS error < 1 photo
    px), then scales the photo's star box to SPAN of the screen width, centred."""

    def __init__(self, w, h):
        self.w, self.h = w, h
        stars = [c for c in CONSTELLATIONS if c[5] is not None]
        v = [_unit(r, d) for *_, r, d in stars]
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
        box_w = min(w, h * 16 / 9)                  # the 16:9 box the layout is sized for
        self.u = box_w / 1920                       # 1 at 1080p: scale for strokes and radii
        self.s = SPAN * box_w / (x1 - x0)
        self.pc = complex((x0 + x1) / 2, (y0 + y1) / 2)
        self.sc = complex(w / 2, h / 2)
        self.px_per_deg = abs(self.a) * self.s * math.pi / 180

    def _gnomonic(self, ra, dec):
        a, d = math.radians(ra) - self.ra0, math.radians(dec)
        cc = math.sin(self.de0) * math.sin(d) + math.cos(self.de0) * math.cos(d) * math.cos(a)
        return complex(math.cos(d) * math.sin(a) / cc,
                       (math.cos(self.de0) * math.sin(d) - math.sin(self.de0) * math.cos(d) * math.cos(a)) / cc)

    def photo(self, x, y):
        """Reference-photo pixel -> screen pixel."""
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

    def lb(self, x, y):
        """Screen pixel -> galactic (l, b), degrees."""
        return eq2gal(*self.radec(x, y))

    def axes(self, ra, dec):
        """Unit screen vectors pointing north and east at (ra, dec)."""
        x, y = self.screen(ra, dec)
        nx, ny = self.screen(ra, dec + 0.05)
        ex, ey = self.screen(ra + 0.05 / math.cos(math.radians(dec)), dec)
        n, e = math.hypot(nx - x, ny - y), math.hypot(ex - x, ey - y)
        return ((nx - x) / n, (ny - y) / n), ((ex - x) / e, (ey - y) / e)


def _unit(ra, dec):
    ra, dec = math.radians(ra), math.radians(dec)
    return math.cos(dec) * math.cos(ra), math.cos(dec) * math.sin(ra), math.sin(dec)


def gal2eq(l, b):
    """Galactic (l, b) -> equatorial (RA, Dec), degrees."""
    an, dn, ln = (math.radians(t) for t in NGP)
    l, b = math.radians(l), math.radians(b)
    d = math.asin(math.sin(dn) * math.sin(b) + math.cos(dn) * math.cos(b) * math.cos(ln - l))
    a = an + math.atan2(math.cos(b) * math.sin(ln - l),
                        math.cos(dn) * math.sin(b) - math.sin(dn) * math.cos(b) * math.cos(ln - l))
    return math.degrees(a) % 360, math.degrees(d)


# --- sky model ---------------------------------------------------------------------

def ifn_strength(l, b):
    """0..1: how much Integrated Flux Nebula a long exposure shows at galactic (l, b)."""
    def dl(l0):                                 # longitude offset, wrapped, in true degrees
        return ((l - l0 + 180) % 360 - 180) * math.cos(math.radians(b))
    b0, fold = IFN_CIRRUS_B
    cirrus = min(1.0, math.exp(-(b - b0) / fold))
    fl, fb, sl, sb, k = POLARIS_FLARE
    flare = k * math.exp(-(dl(fl) / sl) ** 2 - ((b - fb) / sb) ** 2)
    vl, vb, vs, vk = VOLCANO
    volcano = vk * math.exp(-(dl(vl) ** 2 + (b - vb) ** 2) / vs ** 2)
    return min(1.0, 0.55 * cirrus + flare + volcano)


def field_entry(x, y, r, a, bv, sat, glow):
    return [x, y, r, a, bv_rgb(bv, sat), glow, bv, sat]


def field(rng, sky):
    """Background stars: [x, y, r, opacity, colour, glow, B-V, saturation].

    Counts follow a steep luminosity function, as a long exposure records them:
    thousands of faint dots for every bright one, thickest toward the galactic plane.
    """
    w, h, u = sky.w, sky.h, sky.u
    area = w * h / (1920 * 1080)

    def spot():
        while True:
            x, y = rng.uniform(0, w), rng.uniform(0, h)
            if rng.random() < star_density(*sky.lb(x, y)):
                return x, y

    # Arguments are evaluated left to right, so rng is drawn in a fixed order:
    # keep it that way, or every seed gives a different sky.
    out = []
    for _ in range(int(16000 * area)):            # the faint dust: sub-pixel, barely there
        out.append(field_entry(*spot(), (0.26 + 0.24 * rng.random()) * u, 0.18 + 0.40 * rng.random() ** 2,
                               field_bv(rng), 1.6, False))
    for _ in range(int(3200 * area)):             # crisp pin-points
        k = rng.random() ** 3
        out.append(field_entry(*spot(), (0.40 + 0.60 * k) * u, 0.35 + 0.60 * rng.random() ** 1.5,
                               field_bv(rng), 2.2, False))
    for _ in range(int(160 * area)):              # small soft ones, all dimmer than mag 5
        k = rng.random() ** 2
        out.append(field_entry(*spot(), (0.9 + 1.1 * k) * u, 0.22 + 0.32 * rng.random(),
                               field_bv(rng), 2.4, True))
    return out


# --- SVG ---------------------------------------------------------------------------
# Every element uses fill-opacity, never opacity: librsvg renders an element with
# `opacity` through its own offscreen group, which made the ~19k-star layer ~60x slower.

def svg_doc(w, h, defs, body):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">\n'
            f'<defs>{"".join(defs)}</defs>\n{chr(10).join(body)}\n</svg>\n')


def glow_gradient(gid, col, stops):
    """Radial gradient from white at the centre into col; stops = [(offset, opacity)]."""
    s = "".join(f'<stop offset="{o}" stop-color="{col}" stop-opacity="{a}"/>' for o, a in stops)
    return (f'<radialGradient id="{gid}"><stop offset="0" stop-color="#FFFFFF" stop-opacity="0.95"/>'
            f'{s}</radialGradient>')


def field_star(j, star, defs, body):
    x, y, r, a, col, glow, *_ = star
    if not glow:
        body.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r:.2f}" fill="{col}" fill-opacity="{a:.2f}"/>')
        return
    defs.append(glow_gradient(f"f{j}", col, [(0.3, 0.6), (1, 0)]))
    body.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r * 2.2:.2f}" fill="url(#f{j})" fill-opacity="{a:.2f}"/>')


def bright_star(i, x, y, core, halo, col, op, defs, body):
    """A tinted halo plus a crisp core."""
    defs.append(glow_gradient(f"g{i}", col, [(0.18, 0.75), (0.45, 0.18), (1, 0)]))
    body.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{halo:.2f}" fill="url(#g{i})" fill-opacity="{op:.3f}"/>')
    body.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{core:.2f}" fill="{col}" '
                f'fill-opacity="{min(1, 0.35 + op * 0.65):.3f}"/>')


def dipper_star(sky, i, op, defs, body, shift=(0.0, 1.0)):
    """shift = (B-V offset, saturation): (0, 1) is the star's true colour."""
    name, x, y, m, bv, *_ = CONSTELLATIONS[i]
    sx, sy = sky.photo(x, y)
    f, u = mag_scale(m), sky.u
    halo = (3.5 + 15 * f) * u * (UMA_HALO if name in URSA_MAJOR else 1.0)
    bright_star(i, sx, sy, (0.7 + 1.9 * f) * u, halo, bv_rgb(bv + shift[0], shift[1]), op, defs, body)
    return sx, sy


POLARIS = next(i for i, c in enumerate(CONSTELLATIONS) if c[0] == "Polaris")


def polaris(sky, frame, frames, defs, body, shift=(0.0, 1.0)):
    """Polaris, twinkling, with a thin four-point glint that breathes."""
    op = twinkle(1, 0.0, frame, frames, 0.45)
    x, y = dipper_star(sky, POLARIS, op, defs, body, shift)
    arm, u = (10 + 7 * op) * sky.u, sky.u
    body.append(f'<g stroke="{bv_rgb(CONSTELLATIONS[POLARIS][4] + shift[0], shift[1])}" stroke-width="{0.45 * u:.2f}" '
                f'stroke-linecap="round" stroke-opacity="{0.15 + 0.35 * op:.3f}">'
                f'<line x1="{x - arm:.2f}" y1="{y:.2f}" x2="{x + arm:.2f}" y2="{y:.2f}"/>'
                f'<line x1="{x:.2f}" y1="{y - arm:.2f}" x2="{x:.2f}" y2="{y + arm:.2f}"/></g>')


def deep_sky(sky, defs, body):
    """The galaxies and the Owl Nebula at their true position, size and angle."""
    k = sky.px_per_deg / 60                       # px per arcminute
    for i, (name, ra, dec, major, minor, pa, col, op) in enumerate(GALAXIES):
        x, y = sky.screen(ra, dec)
        (nx, ny), (ex, ey) = sky.axes(ra, dec)
        t = math.radians(pa)
        rot = math.degrees(math.atan2(math.cos(t) * ny + math.sin(t) * ey, math.cos(t) * nx + math.sin(t) * ex))
        rx, ry = max(0.7, major * k / 2), max(0.45, minor * k / 2)
        defs.append(glow_gradient(f"gx{i}", col, [(0.2, 0.7), (0.6, 0.22), (1, 0)]))
        g = (f'<g transform="translate({x:.2f} {y:.2f}) rotate({rot:.1f})">'
             f'<ellipse rx="{rx:.2f}" ry="{ry:.2f}" fill="url(#gx{i})" fill-opacity="{op}"/>')
        if name == "M82":                         # its red hydrogen plume, across the disc
            g += f'<ellipse rx="{rx * 0.22:.2f}" ry="{ry * 1.9:.2f}" fill="#FF6A5A" fill-opacity="{0.22 * op:.3f}"/>'
        body.append(g + "</g>")
    ra, dec, size, col = OWL
    x, y = sky.screen(ra, dec)
    defs.append(f'<radialGradient id="owl"><stop offset="0" stop-color="{col}" stop-opacity="0.9"/>'
                f'<stop offset="0.55" stop-color="{col}" stop-opacity="0.55"/>'
                f'<stop offset="1" stop-color="{col}" stop-opacity="0"/></radialGradient>')
    body.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{max(1.3, size * k):.2f}" fill="url(#owl)"/>')


class Scene:
    """The boot sky as transparent SVG layers over the raster sky: a still layer
    (everything that never changes), the twinklers (Polaris and a few background
    stars, which the boot screen animates), Polaris alone, and the Doppler layers."""

    def __init__(self, w, h, seed):
        self.w, self.h = w, h
        self.sky = Sky(w, h)
        rng = random.Random(seed)
        self.stars = field(rng, self.sky)
        soft = [j for j, s in enumerate(self.stars) if s[5] and s[1] < 0.95 * h]
        self.twinklers = {j: (rng.choice((1, 2)), rng.uniform(0, 2 * math.pi))
                          for j in rng.sample(soft, min(FIELD_TWINKLERS, len(soft)))}
        for j in self.twinklers:                  # twinklers peak a little brighter
            self.stars[j][3] = min(1.0, self.stars[j][3] * 1.8)
        # own generator, so adding the Doppler layers left every other image unchanged
        still = [j for j in range(len(self.stars)) if j not in self.twinklers]
        pick = random.Random(f"shift-{seed}")
        self.shifted = sorted(pick.sample(still, round(SHIFT_FRACTION * len(still))))

    def still_layer(self):
        defs, body = [], []
        for j, star in enumerate(self.stars):
            if j not in self.twinklers:
                field_star(j, star, defs, body)
        deep_sky(self.sky, defs, body)
        for i in range(len(CONSTELLATIONS)):
            if i != POLARIS:
                dipper_star(self.sky, i, 1.0, defs, body)
        return svg_doc(self.w, self.h, defs, body)

    def twinkle_layer(self, frame=0, frames=1, with_polaris=True):
        """frames=1 gives every twinkler at rest, for the still PNG."""
        defs, body = [], []
        for j, (cycles, phase) in self.twinklers.items():
            star = list(self.stars[j])
            star[3] *= twinkle(cycles, phase, frame, frames, 0.5)
            field_star(j, star, defs, body)
        if with_polaris:
            polaris(self.sky, frame, frames, defs, body)
        return svg_doc(self.w, self.h, defs, body)

    def polaris_layer(self):
        """Polaris alone, at rest, on a transparent full-size layer."""
        defs, body = [], []
        polaris(self.sky, 0, 1, defs, body)
        return svg_doc(self.w, self.h, defs, body)

    def polaris_box(self):
        """(x, y, w, h) in whole pixels around Polaris's halo and glint."""
        _, x, y, m, *_ = CONSTELLATIONS[POLARIS]
        cx, cy = self.sky.photo(x, y)
        r = math.ceil(max((3.5 + 15 * mag_scale(m)) * self.sky.u, 17 * self.sky.u) + 2)
        x0, y0 = max(0, math.floor(cx) - r), max(0, math.floor(cy) - r)
        return x0, y0, min(self.w, math.floor(cx) + r + 1) - x0, min(self.h, math.floor(cy) + r + 1) - y0

    def shift_layer(self, dbv):
        """The Doppler-shifted stars, B-V moved by dbv: the chosen share of the
        background and every constellation star, Polaris and its glint included."""
        defs, body = [], []
        for j in self.shifted:
            star = list(self.stars[j])
            star[4] = bv_rgb(star[6] + dbv, star[7] * SHIFT_SAT)
            field_star(j, star, defs, body)
        for i in range(len(CONSTELLATIONS)):
            if i != POLARIS:
                dipper_star(self.sky, i, 1.0, defs, body, (dbv, SHIFT_SAT))
        polaris(self.sky, 0, 1, defs, body, (dbv, SHIFT_SAT))
        return svg_doc(self.w, self.h, defs, body)


# --- raster sky --------------------------------------------------------------------
# Drawn with ImageMagick at 16 bits per channel and dithered down to 8, so the dark
# gradient and the faint clouds have no banding and no blocks. (librsvg's
# feTurbulence also leaves square holes at large sizes, so no SVG filters here.)

def fbm(w, h, seed, out, cells=6, octaves=8, lo=44, hi=96, aspect=1.0, angle=0.0):
    """Fractal cloud noise: random grids upscaled smoothly and summed, finest last.

    aspect > 1 stretches the texture into streaks, which are then turned by angle."""
    side = math.ceil(math.hypot(w, h))            # big enough to rotate without empty corners
    args, weights = ["-seed", seed], []
    for k in range(octaves):
        cw, ch = cells * 2 ** k, max(2, round(cells * 2 ** k * aspect))
        args += ["(", "-size", f"{cw}x{ch}", "xc:gray", "+noise", "Random", "-colorspace", "gray",
                 "-filter", "Cubic", "-resize", f"{side}x{side}!", ")"]
        weights.append(f"{0.68 ** k:.4f},1")
    magick(*args, "-poly", " ".join(weights), "-auto-level", "-level", f"{lo}%,{hi}%",
           "-background", "black", "-rotate", f"{angle:.1f}", "+repage",
           "-gravity", "center", "-extent", f"{w}x{h}", out)


def ifn_mask(sky, out, step=8):
    """Grey map of ifn_strength over the screen, computed coarsely, smoothed when used."""
    gw, gh = math.ceil(sky.w / step), math.ceil(sky.h / step)
    px = bytes(round(255 * ifn_strength(*sky.lb((i + 0.5) * step, (j + 0.5) * step)))
               for j in range(gh) for i in range(gw))
    Path(out).write_bytes(f"P5 {gw} {gh} 255\n".encode() + px)


def backdrop(sky, seed, out, tmp, pool):
    """Gradient sky with the Integrated Flux Nebula screened on, dithered to 8 bits.

    The dust's fractal texture is stretched 2:1 along the direction of galactic
    latitude (as the Polaris Flare's filaments run) and shown where ifn_strength is."""
    w, h = sky.w, sky.h
    fx, fy = sky.screen(*gal2eq(POLARIS_FLARE[0], POLARIS_FLARE[1]))
    gx, gy = sky.screen(*gal2eq(POLARIS_FLARE[0], POLARIS_FLARE[1] + 5))
    angle = math.degrees(math.atan2(gy - fy, gx - fx))
    noises = [f"{tmp}/ifn{k}.png" for _, _, k in IFN_LAYERS]
    jobs = [pool.submit(fbm, w, h, seed + k, n, cells=4 + k, aspect=2.0, angle=angle, lo=40, hi=97)
            for (_, _, k), n in zip(IFN_LAYERS, noises)]
    mask = f"{tmp}/ifn.pgm"
    ifn_mask(sky, mask)
    for j in jobs:
        j.result()
    args = ["-size", f"{w}x{h}", f"gradient:{BG_TOP}-{BG_BOTTOM}",
            "(", mask, "-filter", "Cubic", "-resize", f"{w}x{h}!", "-write", "mpr:mask", "+delete", ")"]
    for (col, op, _), noise in zip(IFN_LAYERS, noises):
        args += ["(", "-size", f"{w}x{h}", f"xc:{col}",
                 "(", noise, "mpr:mask", "-compose", "Multiply", "-composite", "-colorspace", "sRGB", ")",
                 "-compose", "Multiply", "-composite", "-evaluate", "multiply", op, ")",
                 "-compose", "Screen", "-composite"]
    magick(*args, "-seed", seed, "-attenuate", DITHER, "+noise", "Uniform", "-depth", 8, out)


# Leave ImageMagick's date chunks out of final PNGs, so a re-render with the same
# options gives byte-identical files (and no spurious git diffs).
PNG_NO_DATES = ("-define", "png:exclude-chunks=date,time")


# --- frames ----------------------------------------------------------------------

@contextmanager
def workspace():
    """(tmp, pool): a scratch folder and a thread pool for the renderers."""
    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(os.cpu_count() or 4) as pool:
        yield tmp, pool


def flatten(base, layer, out):
    """A final PNG: layer over base, without date chunks."""
    magick(base, layer, "-composite", *PNG_NO_DATES, out)


def render_base(scene, seed, out, tmp, pool):
    """The raster sky with every still element on it: the image all frames share."""
    sky, layer = f"{tmp}/sky.png", f"{tmp}/still.png"
    job = pool.submit(render, scene.still_layer(), scene.w, scene.h, layer)
    backdrop(scene.sky, seed, sky, tmp, pool)
    job.result()
    magick(sky, layer, "-composite", out)
