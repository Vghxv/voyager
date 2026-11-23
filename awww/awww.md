# voyager — wallpaper (awww)

The desktop wallpaper, animated: the Plymouth background gradient, twinkling stars, a faint indigo/teal nebula, and the probe in the lower-right sending a signal pulse through its rings toward a pale blue dot. The center is kept calm, because windows cover it and the lock screen puts its probe there.

![wallpaper](wallpaper.webp)

It runs on [awww](https://codeberg.org/LGFae/awww) (the renamed swww).

## Install

Needs a systemd-managed graphical session: Hyprland started through [uwsm](https://github.com/Vladimir-csp/uwsm) (`uwsm start hyprland.desktop`). `awww.service` starts with `graphical-session.target`, which plain `Hyprland` from a TTY never activates. The installer checks this and stops if it is missing.

```bash
sudo pacman -S awww
./awww/install.sh               # as your user, not sudo — animated (48 frames, 12 fps, 4 s loop)
./awww/install.sh --static      # still image instead, no animation cost
./awww/install.sh --restore     # stop and disable awww
```

What the installer does:

- Copies `wallpaper.webp` and `wallpaper.png` to `~/.config/hypr/voyager/`.
- Installs `awww.service` (the package ships none) to `~/.config/systemd/user/` and enables it. An existing unit with the same name is backed up first.
- `awww img … --transition-type fade` shows the wallpaper. awww caches it and shows it again at next login.

`--restore` stops and disables `awww.service`. awww is the only wallpaper daemon voyager uses: hyprpaper is not needed and the installer never touches it.

Check what is shown: `awww query`.

## Cost

Measured on Intel Iris Xe with two 1920×1080 monitors: about **3–4 % of one CPU core** and **~37 MB of memory** while animating. `--static` costs nothing after the fade. To pause the animation while the screens are off, add `awww pause` / `awww unpause` to the DPMS listener in `hypridle.conf`:

```ini
listener {
    timeout = 330
    on-timeout = awww pause; hyprctl dispatch 'hl.dsp.dpms({ action = "disable" })'
    on-resume  = hyprctl dispatch 'hl.dsp.dpms({ action = "enable" })'; awww unpause
}
```

## Re-generate

`src/gen-wallpaper.py` builds both files from SVG (Python stdlib, `rsvg-convert`, ImageMagick). The probe comes from `src/logo.svg`. It takes about a minute.

```bash
src/gen-wallpaper.py                          # 1920x1080 → awww/wallpaper.webp + awww/wallpaper.png
src/gen-wallpaper.py --fps 8 --frames 32      # lighter animation, same 4 s loop
src/gen-wallpaper.py --size 3840x2160         # 4K
src/gen-wallpaper.py --seed 42                # different star layout
./awww/install.sh                             # install the new files
```

Colors, nebula placement, twinkle strength and ring pulse are constants at the top of the script.
