# voyager

A deep-space theme for an Arch + Hyprland desktop: a probe drifting through a starfield, from the disk unlock at boot to the lock screen and the wallpaper.

![preview](docs/preview.gif)

| Component | What you get | Install | Undo |
|---|---|---|---|
| [Plymouth](plymouth/plymouth.md) | Animated disk-unlock screen in the voyager sky: stars warp, blue-shift and signal rings pulse as you type | `sudo ./plymouth/install.sh` | [see docs](plymouth/plymouth.md#remove) |
| [hyprlock](hyprlock/hyprlock.md) | Lock screen over the same sky as the wallpaper, with ✦ password dots | `./hyprlock/install.sh` | `--restore` |
| [Wallpaper](awww/awww.md) | The Big and Little Dipper around a twinkling Polaris, or a still image | `./awww/install.sh` | `--restore` |
| [eww](eww/eww.md) | Top bar on every monitor: workspaces, clock, wifi, volume, brightness, webcam, battery, tray | `./eww/install.sh` | — (one-way copy) |
| [dunst](dunst/dunst.md) | Notifications styled like the bar | `./dunst/install.sh` | `--restore` |

Each component installs on its own; pick the ones you want. Installers that replace a config file back it up first, and `--restore` puts it back (for the wallpaper: stops awww and puts back your own `awww.service`, if you had one). Plymouth needs some boot setup first: read its [prerequisites](plymouth/plymouth.md#prerequisites).

## When something breaks

```bash
./doctor.sh            # every component
./doctor.sh eww        # just one
```

It changes nothing. For each component it checks the packages, whether the installed files still match this repo, and whether the daemons and bars are up. Every ✗ comes with the command that fixes it. Each component's doc also ends with an **If it breaks** table.

## Light and dark

The bar and the notifications have a light and a dark mode and switch together:

```bash
~/.config/scripts/voyager-theme toggle       # flip
~/.config/scripts/voyager-theme light|dark   # set
~/.config/scripts/voyager-theme              # show the current mode
```

Bind `toggle` to a key, e.g. in `hyprland.lua`:

```lua
hl.bind("SUPER + T", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/scripts/voyager-theme toggle"))
```

If eww is not on Hyprland's `PATH`, start the command with `EWW=/path/to/eww `.

The mode is saved in `~/.config/voyager/theme`. Switching runs one small hook per component from `~/.config/voyager/theme.d/`, which the eww and dunst installers put there. If a hook fails, the others still run and the error tells you how to retry it.

## How the pieces fit

The components share a look but do not depend on each other. Each folder holds its files, an `install.sh`, a `check.sh` (used by `doctor.sh`) and a `deps` file listing the commands it needs.

```
plymouth/    boot unlock screen     (preview.sh tries it without rebooting)
hyprlock/    lock screen
awww/        wallpaper + systemd unit
eww/         the bar
dunst/       notifications
lib/         helpers shared by the scripts, voyager-theme
src/         SVG artwork and the generators that render every image
docs/        preview.gif, recorded by src/gen-plymouth-preview.py
doctor.sh    runs every check.sh
```

A few things are kept in step by hand, because they change rarely:

- **Palette** (navy `#14141F`, mist `#D9D9F2`, indigo `#8B8FF0`, teal `#5EEAD4`, red `#FF3355`): repeated in `eww/eww.scss`, `dunst/themes/`, `hyprlock/hyprlock.conf`, `plymouth/voyager/voyager.script` and `src/voyager_art.py`.
- **Probe dish position**: in `src/voyager_art.py` and `voyager.script`. Both comments point at each other.
- **dunst `offset`**: it leaves room for the eww bar's height.
