#!/usr/bin/env python3
"""Record docs/preview.gif from the installed Plymouth theme.

Run it from a text console (Ctrl+Alt+F3), not a terminal window or tmux. It
shows the splash on the real screen, records it with ffmpeg's kmsgrab, and
you type the demo passphrases (thrown away). Needs plymouth and ffmpeg.
Stuck? Ctrl+C, then: sudo plymouth quit
"""

import glob
import os
import signal
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True          # no src/__pycache__ (and none owned by root)
from voyager_art import ROOT  # noqa: E402

OUT = ROOT / "docs" / "preview.gif"
STEPS = """\
Once the splash is up:
  1. wait for the hint, type 6 characters slowly
  2. Backspace twice, then Enter       -> warp
  3. it asks again                     -> red flash (looks like a wrong passphrase)
  4. type 7 characters, then Enter     -> warp, then it ends
"""


def run(*cmd, **kw):
    # stdin closed: only plymouthd may read the keyboard
    return subprocess.run(cmd, stdin=subprocess.DEVNULL, check=True, **kw)


def record(tty, raw):
    run("plymouthd", f"--tty={tty}")    # it reads tty1 by default
    run("plymouth", "show-splash")
    # Plymouth draws into a linear buffer, so kmsgrab frames copy out directly
    rec = subprocess.Popen(["ffmpeg", "-nostdin", "-loglevel", "error", "-device", glob.glob("/dev/dri/card*")[0],
                            "-f", "kmsgrab", "-framerate", "25", "-i", "-", "-vf", "hwdownload,format=bgr0",
                            "-c:v", "libx264", "-preset", "ultrafast", "-crf", "10", "-pix_fmt", "yuv444p", raw],
                           stdin=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    try:
        time.sleep(2.5)
        if rec.poll() is not None:
            raise SystemExit(f"ffmpeg could not record the screen:\n{rec.stderr.read()}")
        for _ in range(2):              # asked again after Enter = "wrong passphrase"
            run("plymouth", "ask-for-password", stdout=subprocess.DEVNULL)
            time.sleep(1)
        time.sleep(1.5)
    finally:
        rec.send_signal(signal.SIGINT)
        rec.wait()
        subprocess.run(["plymouth", "quit"])


if os.geteuid() != 0:
    tty = os.ttyname(0) if os.isatty(0) else "a pipe"
    if not tty[8:].isdigit():
        sys.exit(f"Run this from a text console (Ctrl+Alt+F3), not from {tty}.")
    if subprocess.run(["plymouth-set-default-theme"], capture_output=True, text=True).stdout.strip() != "voyager":
        sys.exit("Make voyager the default theme: sudo ./plymouth/install.sh")
    print(STEPS)
    input("Press Enter to start… ")
    subprocess.run(["sudo", "-v"], check=True)
    # One sudo, with no input: sudo runs its command on a new pty and copies
    # its input there, which would steal keystrokes from plymouthd
    os.dup2(os.open(os.devnull, os.O_RDONLY), 0)
    os.execvp("sudo", ["sudo", sys.executable, __file__, tty])

with tempfile.TemporaryDirectory() as work:
    raw = f"{work}/raw.mkv"
    record(sys.argv[1], raw)
    run("ffmpeg", "-loglevel", "error", "-y", "-i", raw, "-vf",
        "scale=640:-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];"
        "[b][p]paletteuse=dither=bayer:bayer_scale=4", OUT)
    os.chown(OUT, int(os.environ["SUDO_UID"]), int(os.environ["SUDO_GID"]))
print(f"wrote {OUT}")
