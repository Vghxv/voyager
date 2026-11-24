# Wallpaper (awww)

The desktop wallpaper: the boot screen's gradient, twinkling stars, a faint nebula, and the probe in the lower right sending a signal pulse toward a pale blue dot. The center stays calm, because windows cover it and the lock screen puts its probe there.

![wallpaper](wallpaper.webp)

It is shown by [awww](https://codeberg.org/LGFae/awww), the renamed swww, running as a systemd user service.

## Install

You need Hyprland started through [uwsm](https://github.com/Vladimir-csp/uwsm) (`uwsm start hyprland.desktop`). The service starts with your graphical session, and plain `Hyprland` from a TTY never starts one. The installer checks this.

```bash
sudo pacman -S awww
./awww/install.sh               # animated
./awww/install.sh --static      # still image, no ongoing CPU cost
./awww/install.sh --restore     # stop and disable awww
```

awww is the only wallpaper daemon voyager uses. You do not need hyprpaper, and the installer leaves it alone.

## Cost

On Intel Iris Xe with two 1920×1080 monitors the animation uses about **3–4 % of one CPU core** and **~37 MB of memory**. `--static` uses nothing once shown. To pause the animation while the screens are off, add `awww pause` / `awww unpause` to the screen-off listener in `hypridle.conf`:

```ini
listener {
    timeout = 330
    on-timeout = awww pause; hyprctl dispatch 'hl.dsp.dpms({ action = "disable" })'
    on-resume  = hyprctl dispatch 'hl.dsp.dpms({ action = "enable" })'; awww unpause
}
```

## Re-render

To make it sharper on a bigger screen, lighter, or to get a different star layout, re-render it. This takes about a minute.

```bash
src/gen-wallpaper.py --size auto     # match your largest monitor; other options: --help
./awww/install.sh
```

## If it breaks

Start with `./doctor.sh awww`.

| Symptom | Do this |
|---|---|
| Installer says "No systemd graphical session" | start Hyprland with `uwsm start hyprland.desktop` |
| No wallpaper after login | `systemctl --user restart awww`; why: `journalctl --user -u awww -n 20` |
| Old image after re-rendering | `./awww/install.sh` (it clears awww's cache) |
| Looks soft or blurry | re-render with `--size auto` (see above) |
