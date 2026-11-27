# voyager

A deep-space theme for an Arch + Hyprland desktop: a probe drifting through a starfield, from the disk unlock at boot to the lock screen and the wallpaper.

![preview](docs/preview.gif)

| Component | What you get | Install | Undo |
|---|---|---|---|
| [Plymouth](plymouth/plymouth.md) | Animated disk-unlock screen in the voyager sky: stars warp, blue-shift and signal rings pulse as you type | `sudo ./plymouth/install.sh` | [see docs](plymouth/plymouth.md#remove) |
| [hyprlock](hyprlock/hyprlock.md) | Lock screen over the same sky as the wallpaper, with ✦ password dots | `./hyprlock/install.sh` (after the wallpaper) | `--restore` |
| [Wallpaper](awww/awww.md) | The real sky around Voyager 1's true position, with every constellation star at its true brightness and color and the Milky Way; Voyager 2 is drawn at its true position too. In drag mode you turn the sky (lines, names in 中文/English) and set the view you like | `./awww/install.sh` | `--restore` |
| [eww](eww/eww.md) | Top bar on every monitor: workspaces, clock, wifi, volume, brightness, webcam, battery, tray | `./eww/install.sh` | — (one-way copy) |
| [dunst](dunst/dunst.md) | Notifications styled like the bar | `./dunst/install.sh` | `--restore` |

The images are stored with [Git LFS](https://git-lfs.com/): install it (`sudo pacman -S git-lfs`, then `git lfs install`) before cloning, or run `git lfs pull` in a clone made without it. The installers stop if an image is still an LFS pointer.

The constellation drawings in `awww/sky/figures/` are Johan Meuris's art from Stellarium, under the Free Art License 1.3 (see its `LICENSE.md`); everything else is MIT.

Each component installs on its own; pick the ones you want. Installers that replace a config file back it up first. Plymouth needs some boot setup first: read its [prerequisites](plymouth/plymouth.md#prerequisites).

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
~/.config/scripts/voyager-theme apply        # re-apply the current mode
~/.config/scripts/voyager-theme              # show the current mode
```

Bind `toggle` to a key, e.g. in `hyprland.lua`:

```lua
hl.bind("SUPER + T", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/scripts/voyager-theme toggle"))
```

If eww is not on Hyprland's `PATH`, start the command with `EWW=/path/to/eww `.

The mode is saved in `~/.config/voyager/theme`. Switching runs one small hook per component from `~/.config/voyager/theme.d/`, which the eww and dunst installers put there. If a hook fails, the others still run and the error tells you how to retry it.

## How the pieces fit

The components share a look and work on their own, with one link: the lock screen shows the wallpaper's sky. Each time voyager-sky sets the wallpaper (install, `voyager-sky apply`, leaving drag mode) it also writes `~/.config/hypr/voyager/lock.png`, the same view without the small probe, and hyprlock uses that as its background. Without the wallpaper, the lock screen is plain navy.

Each folder holds its files, an `install.sh`, a `check.sh` (used by `doctor.sh`) and a `deps` file listing the commands it needs.

```
plymouth/    boot unlock screen     (preview.sh tries it without rebooting)
hyprlock/    lock screen
awww/        wallpaper: voyager-sky (Rust, drag mode + renderer), sky data, systemd unit
eww/         the bar
dunst/       notifications
lib/         helpers shared by the scripts, voyager-theme
src/         the Plymouth generators; art/ the SVG artwork, sky/ the scripts that build awww/sky
docs/        preview.gif, recorded by src/gen-plymouth-preview.py
doctor.sh    runs every check.sh
```

A few things are kept in step by hand, because they change rarely:

- **Palette** (navy `#14141F`, mist `#D9D9F2`, indigo `#8B8FF0`, teal `#5EEAD4`, red `#FF3355`, and a few shades): defined in [`lib/palette`](lib/palette), which also lists every file that keeps a copy (eww, dunst, hyprlock, the Plymouth script, the SVGs). `src/voyager_art.py` and `awww/voyager-sky` (compiled in) read it directly; `./doctor.sh` checks the copies still match.
- **dunst `offset`**: its `50` leaves room for the eww bar's height.

## Power use

The wallpaper is a still image, and `voyager-sky listen` waiting for a long-press uses no CPU or GPU. [Drag mode](awww/awww.md#drag-mode) costs power only while it draws, which it does whenever something on the sky moves. Measured on an i5-1135G7 (Iris Xe), one 1920×1080 60 Hz screen, on AC. Each figure is the average over 3 minutes after the sky settled:

| Drag mode | What keeps it drawing | Package power | Over drag mode off | iGPU time | CPU time |
|---|---|---|---|---|---|
| Off (`listen` waiting) | nothing | 1.7 W | — | 0 % | 0 % |
| On, Turntable, toggles off | nothing | 1.6 W | ~0 | 0 % | 0 % |
| On, Turntable, lines on (`C`) | the lines' flicker, at 12 fps | 2.2 W | +0.5 W | 4 % | 1 % |
| On, Trackball, toggles off | the zodiac tour | 3.3 W | +1.6 W | 19 % | 5 % |
| On, Trackball, `C` `F` `E` `Q` on | the tour, every overlay | 3.5 W | +1.7 W | 24 % | 9 % |

Package power is the CPU and iGPU together (RAPL), not the screen or the rest of the laptop. The iGPU and CPU columns are the share of time `voyager-sky` used them.

- **Drawing is what costs, not drag mode being open.** Turning the lines, figures, ecliptic and equator on adds only about 0.2 W to a sky that is already redrawing.
- **The zodiac tour draws every frame.** In Trackball it starts 5 s after you stop touching drag mode, so drag mode left open in Trackball soon costs about +1.6 W.
- **The flicker is cheap on its own.** It is drawn at 12 fps when nothing else moves. Before that it was drawn every frame: 3.3 W, iGPU 19 %.
- **To leave drag mode open cheaply**, use Turntable (`T`) with the lines off.

A second monitor will cost more, since drag mode draws on every screen; it has not been measured. The tour rows were measured before the 12 fps change; the tour still draws every frame.
