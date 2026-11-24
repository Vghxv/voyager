#!/usr/bin/env python3
"""Shrink awww/wallpaper.webp into docs/wallpaper.gif. Needs magick."""

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
subprocess.run(["magick", ROOT / "awww/wallpaper.webp", "-coalesce", "-resize", "640x",
                "+dither", "-colors", "128", "-layers", "Optimize", ROOT / "docs/wallpaper.gif"], check=True)
