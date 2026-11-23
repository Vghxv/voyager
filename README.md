# voyager

A deep-space theme for an Arch + Hyprland desktop: a probe drifting through a starfield, from LUKS unlock to lock screen to wallpaper.

![preview](docs/preview.gif)

| Component | What it is | Install | Docs |
|---|---|---|---|
| **Plymouth** | Animated LUKS unlock: stars warp and signal rings pulse as you type | `sudo ./plymouth/install.sh` | [plymouth.md](plymouth/plymouth.md) |
| **hyprlock** | Lock screen with a crossfading starfield and ✦ password dots | `./hyprlock/install.sh` | [hyprlock.md](hyprlock/hyprlock.md) |
| **Wallpaper** | Animated wallpaper (twinkling stars, signal pulse) via awww; `--static` for a still | `./awww/install.sh` | [awww.md](awww/awww.md) |
| **eww** | Top bar: workspaces, clock, wifi, volume, brightness, webcam, battery, tray — light mist cells, teal/amber/red states | `./eww/install.sh` | [eww.md](eww/eww.md) |
| **dunst** | Notifications as mist panels under the bar: indigo edge, teal progress bar, red for critical | `./dunst/install.sh` | [dunst.md](dunst/dunst.md) |

Plymouth needs a systemd initramfs with `sd-encrypt` — read its [prerequisites](plymouth/plymouth.md#prerequisites) first. The plymouth, hyprlock, awww and dunst installers back up what they replace; hyprlock, awww and dunst undo with `--restore`. eww deploys one way (repo → `~/.config/eww`), with no backup.

```
plymouth/    boot theme, install.sh, preview.sh
hyprlock/    lock screen config + assets, install.sh
awww/        animated + still wallpaper, systemd unit, install.sh
eww/         complete bar: yuck, scripts, eww.scss, icons, install.sh
dunst/       notification config, install.sh
lib/         shared shell helpers
src/         SVG sources, wallpaper + hyprlock + plymouth generators
docs/        preview images
```

[MIT](LICENSE)
