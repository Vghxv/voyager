# voyager

A deep-space theme for an Arch + Hyprland desktop: a probe drifting through a starfield, from LUKS unlock to lock screen to wallpaper.

![preview](docs/preview.gif)

| Component | What it is | Install | Docs |
|---|---|---|---|
| **Plymouth** | Animated LUKS unlock: stars warp and signal rings pulse as you type | `sudo ./plymouth/install.sh` | [plymouth.md](plymouth/plymouth.md) |
| **hyprlock** | Lock screen with a crossfading starfield and ✦ password dots | `./hyprlock/install.sh` | [hyprlock.md](hyprlock/hyprlock.md) |
| **Wallpaper** | Animated wallpaper (twinkling stars, signal pulse) via awww; `--static` for a still | `./awww/install.sh` | [awww.md](awww/awww.md) |
| **eww** | Bar restyle: light mist cells, navy icons, teal/amber/red states. Your yuck is untouched | `./eww/install.sh` | [eww.md](eww/eww.md) |

Plymouth needs a systemd initramfs with `sd-encrypt` — read its [prerequisites](plymouth/plymouth.md#prerequisites) first. The plymouth, hyprlock and awww installers back up what they replace; hyprlock and awww undo with `--restore`. eww deploys one way (repo → `~/.config/eww`), with no backup.

```
plymouth/    boot theme, install.sh, preview.sh
hyprlock/    lock screen config + assets, install.sh
awww/        animated + still wallpaper, systemd unit, install.sh
eww/         bar eww.scss, navy icons, install.sh
lib/         shared shell helpers
src/         SVG sources + wallpaper generator
docs/        preview images
```

[MIT](LICENSE)
