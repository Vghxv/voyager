#!/usr/bin/env python3
"""Render docs/hyprlock.png from hyprlock/stars-0.png and probe.png, laid out
like hyprlock/hyprlock.conf (keep the two in step). Needs rsvg-convert."""

import base64
import struct
import sys

sys.dont_write_bytecode = True
import voyager_art as art  # noqa: E402

SRC = art.ROOT / "hyprlock"


def png(name):
    return "data:image/png;base64," + base64.b64encode((SRC / name).read_bytes()).decode()


w, h = struct.unpack(">II", (SRC / "stars-0.png").read_bytes()[16:24])
cx, cy = w / 2, h / 2                   # hyprlock positions: offsets from centre, y up
pt = 4 / 3                              # hyprlock font_size is in points
svg = f"""<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}">
<linearGradient id="o" x1="0" y1="1" x2="1" y2="0">
  <stop stop-color="{art.TEAL}" stop-opacity=".55"/><stop offset="1" stop-color="{art.INDIGO}" stop-opacity=".55"/>
</linearGradient>
<image xlink:href="{png('stars-0.png')}" width="{w}" height="{h}"/>
<image xlink:href="{png('probe.png')}" x="{cx - 130}" y="{cy - 320}" width="260" height="260"/>
<g font-family="DejaVu Sans" text-anchor="middle">
  <text x="{cx}" y="{cy - 8}" font-size="{26 * pt}" fill="{art.MIST}">voyager</text>
  <text x="{cx}" y="{cy + 18}" font-size="{14 * pt}" fill="#9999B3">22:14</text>
  <rect x="{cx - 209}" y="{cy + 55}" width="418" height="50" rx="25"
        fill="#0A0A14" fill-opacity=".55" stroke="url(#o)" stroke-width="2"/>
  <text x="{cx}" y="{cy + 86}" font-size="{14 * pt}" font-style="italic" fill="#8080A0">password, then enter to launch</text>
</g>
</svg>"""
art.render(svg, w, h, art.ROOT / "docs" / "hyprlock.png")
