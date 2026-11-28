#!/usr/bin/env python3
"""Record docs/preview.gif from the installed Plymouth theme, hands-free.

Run it from a text console (Ctrl+Alt+F3), not a terminal window or tmux. It
shows the splash on the real screen, records it with ffmpeg's kmsgrab, and
types the demo passphrases itself on a virtual keyboard (/dev/uinput), timed
so each effect reads clearly: blue shift, red shift, warp, and the probe's red
flash on a wrong passphrase. Don't touch the keyboard while it runs.
Needs plymouth and ffmpeg. Stuck? Ctrl+C, then: sudo plymouth quit
"""

import fcntl
import glob
import os
import signal
import struct
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True          # no src/__pycache__ (and none owned by root)
from voyager_art import ROOT  # noqa: E402

OUT = ROOT / "docs" / "preview.gif"


def run(*cmd, **kw):
    # stdin closed: only plymouthd may read the keyboard
    return subprocess.run(cmd, stdin=subprocess.DEVNULL, check=True, **kw)


class Keyboard:
    """A virtual keyboard: its keys reach the active console like real ones."""

    # linux/uinput.h and linux/input-event-codes.h
    UI_SET_EVBIT, UI_SET_KEYBIT = 0x40045564, 0x40045565
    UI_DEV_SETUP, UI_DEV_CREATE, UI_DEV_DESTROY = 0x405C5503, 0x5501, 0x5502
    EV_SYN, EV_KEY = 0, 1
    BUS_VIRTUAL = 6
    LETTERS = [30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44]
    BACKSPACE, ENTER = 14, 28

    def __init__(self):
        if not os.path.exists("/dev/uinput"):
            run("modprobe", "uinput")
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        fcntl.ioctl(self.fd, self.UI_SET_EVBIT, self.EV_KEY)
        for code in self.LETTERS + [self.BACKSPACE, self.ENTER]:
            fcntl.ioctl(self.fd, self.UI_SET_KEYBIT, code)
        fcntl.ioctl(self.fd, self.UI_DEV_SETUP,
                    struct.pack("4H80sI", self.BUS_VIRTUAL, 0x1234, 0x5678, 1, b"voyager-preview", 0))
        fcntl.ioctl(self.fd, self.UI_DEV_CREATE)
        time.sleep(1)                   # let the console's keyboard handler attach
        self.n = 0

    def _emit(self, typ, code, value):
        os.write(self.fd, struct.pack("llHHi", 0, 0, typ, code, value))

    def tap(self, code):
        for value in (1, 0):            # press, release
            self._emit(self.EV_KEY, code, value)
            self._emit(self.EV_SYN, 0, 0)
            time.sleep(0.01)

    def type(self, count, gap):
        for _ in range(count):          # what it types doesn't matter: no bullets are drawn
            self.tap(self.LETTERS[self.n % len(self.LETTERS)])
            self.n += 1
            time.sleep(gap)

    def backspace(self, count, gap):
        for _ in range(count):
            self.tap(self.BACKSPACE)
            time.sleep(gap)

    def close(self):
        fcntl.ioctl(self.fd, self.UI_DEV_DESTROY)
        os.close(self.fd)


def ask():
    # Returns once Enter is pressed; the "passphrase" it prints is thrown away
    return subprocess.Popen(["plymouth", "ask-for-password"], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL)


def perform(kb):
    """The demo. The theme's speed decays fast, so keys come in quick bursts."""
    time.sleep(2.5)                     # idle sky, the hint fades in
    prompt = ask()
    time.sleep(0.8)
    kb.type(5, 0.07)                    # burst forward: warp stars + blue shift
    time.sleep(1.3)
    kb.type(3, 0.07)
    time.sleep(1.2)
    kb.backspace(4, 0.08)               # pull back: stars reverse + red shift
    time.sleep(1.4)
    kb.type(3, 0.08)
    time.sleep(0.4)
    kb.tap(kb.ENTER)                    # launch: the warp ramps up, the sky goes blue
    prompt.wait()
    time.sleep(2.2)
    prompt = ask()                      # asked again = wrong passphrase: the probe flashes red, shakes
    time.sleep(2.5)                     # the red fades and the hint comes back
    kb.type(6, 0.08)
    time.sleep(0.5)
    kb.tap(kb.ENTER)                    # right this time: warp out
    prompt.wait()
    time.sleep(2.5)


def record(tty, raw):
    kb = Keyboard()
    run("plymouthd", f"--tty={tty}")    # it reads tty1 by default
    run("plymouth", "show-splash")
    # Plymouth draws into a linear buffer, so kmsgrab frames copy out directly
    rec = subprocess.Popen(["ffmpeg", "-nostdin", "-loglevel", "error", "-device", glob.glob("/dev/dri/card*")[0],
                            "-f", "kmsgrab", "-framerate", "25", "-i", "-", "-vf", "hwdownload,format=bgr0",
                            "-c:v", "libx264", "-preset", "ultrafast", "-crf", "10", "-pix_fmt", "yuv444p", raw],
                           stdin=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    try:
        time.sleep(0.5)
        if rec.poll() is not None:
            raise SystemExit(f"ffmpeg could not record the screen:\n{rec.stderr.read()}")
        perform(kb)
    finally:
        rec.send_signal(signal.SIGINT)
        rec.wait()
        subprocess.run(["plymouth", "quit"])
        kb.close()


if os.geteuid() != 0:
    tty = os.ttyname(0) if os.isatty(0) else "a pipe"
    if not tty[8:].isdigit():
        sys.exit(f"Run this from a text console (Ctrl+Alt+F3), not from {tty}.")
    if subprocess.run(["plymouth-set-default-theme"], capture_output=True, text=True).stdout.strip() != "voyager":
        sys.exit("Make voyager the default theme: sudo ./plymouth/install.sh")
    subprocess.run(["sudo", "-v"], check=True)
    print("Recording for ~25 s, hands off the keyboard…")
    # One sudo, with no input: sudo runs its command on a new pty and copies
    # its input there, which would steal keystrokes from plymouthd
    os.dup2(os.open(os.devnull, os.O_RDONLY), 0)
    os.execvp("sudo", ["sudo", sys.executable, __file__, tty])

with tempfile.TemporaryDirectory() as work:
    raw = f"{work}/raw.mkv"
    record(sys.argv[1], raw)
    # Full screen resolution, no scaling. The palette comes from every pixel
    # (stats_mode=full), not only the moving ones, or the dark nebula has too
    # few shades. Error diffusion keeps its gradients smooth where Bayer leaves
    # a grid, and diff_mode=rectangle re-dithers only what changed, so the
    # still sky doesn't shimmer.
    run("ffmpeg", "-loglevel", "error", "-y", "-i", raw, "-vf",
        "split[a][b];[a]palettegen=stats_mode=full[p];"
        "[b][p]paletteuse=dither=sierra2_4a:diff_mode=rectangle", OUT)
    os.chown(OUT, int(os.environ["SUDO_UID"]), int(os.environ["SUDO_GID"]))
print(f"wrote {OUT}")
