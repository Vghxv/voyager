# Wallpaper (awww)

The desktop wallpaper: the boot screen's gradient with Ursa Major and Ursa Minor in the middle, arranged as they appear in a wide-field photo of the two. The Big Dipper's pointer stars, Merak and Dubhe, aim at Polaris. Each star is drawn at its true brightness and colour, which is why Dubhe and Kochab are orange. Behind them is what a long exposure of this part of the sky really shows, placed from real sky coordinates:

- **Integrated Flux Nebula**: faint grey-brown dust lit by the combined light of the Milky Way. It thickens toward the galactic plane (the right of the screen), with the **Polaris Flare** beside Polaris and the dust cloud around M81 and M82.
- **Owl Nebula (M97)**: a tiny teal planetary nebula next to Merak.
- **Galaxies** at their true size and angle: M81 and M82 (with its faint red plume), the Pinwheel M101, M108, M109 and NGC 6217.
- Thousands of faint blue, white and yellow stars, also thickening toward the galactic plane.

A tiny probe drifts in the lower right. In the animated version only Polaris and five faint background stars twinkle.

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

To make it sharper on a bigger screen, lighter, or to get a different texture for the faint stars and dust, re-render it. This takes about two minutes. The constellations always stay the same.

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
