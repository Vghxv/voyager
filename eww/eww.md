# voyager — eww bar

A complete [eww](https://github.com/elkowar/eww) top bar, on every monitor, in the voyager palette, styled as a HUD: square cells (no rounded corners), monospace text, a thin indigo frame with a 2px accent edge along the bottom, teal for "ok" states (wifi connected, charging, camera on), amber for mid battery, and the hyprlock fail red for low battery or no wifi. It comes in a light mode (mist cells, navy text) and a dark mode (navy glass cells, mist text): see [Light and dark](#light-and-dark).

Left: power menu, clock, date, Hyprland workspaces (click one to switch to it; each bar highlights the workspace on its own monitor). Right: wifi, volume slider, mixer (pavucontrol), brightness, webcam toggle, battery (tooltip: time left), system tray.

## Install

```bash
sudo pacman -S jq socat inotify-tools libpulse wireplumber pavucontrol   # what the bar's scripts call
                                                                          # + networkmanager or iw, for wifi
./eww/install.sh             # as your user, not sudo
```

`eww` itself must be on `PATH`. Arch ships it in the AUR (`eww`), or build it from source.

The installer copies the whole bar into `~/.config/eww`: `eww.yuck`, `var.yuck`, `utils.yuck`, `audio/`, `scripts/`, `eww.scss`, `themes/`, the icons into `assets/light/` and `assets/dark/`, and the saved mode as `theme.scss`. It also installs the mode switcher to `~/.local/bin/voyager-theme`. It reports any missing packages and reloads eww if it is running.

Deployment is one way: repo → `~/.config/eww`. Nothing is read back and no backups are made. Files with the same name are overwritten, and other files in `~/.config/eww` are left alone. Edit the files here and run the installer again.

Start the bar, and start it at login. `scripts/bars.sh` opens one `bar` window per monitor (id `bar-<monitor>`), starting the eww daemon if needed; `eww open bar` alone no longer works, because the window needs to know its monitor.

```bash
~/.config/eww/scripts/bars.sh                                  # now
~/.config/eww/scripts/bars.sh toggle                           # hide/show every bar (for a keybind)
exec-once = ~/.config/eww/scripts/bars.sh watch                # hyprland.conf: open, then follow monitor hotplug
hl.exec_cmd(os.getenv("HOME") .. "/.config/eww/scripts/bars.sh watch")   -- hyprland.lua, in hyprland.start
```

If `eww` is not on `PATH`, pass it: `EWW=/path/to/eww bars.sh`.

## Machine-specific settings

Nothing needs editing: the bar detects the machine it runs on.

- **Battery**: `scripts/battery_name.sh` picks the first power supply of type Battery, skipping mouse and keyboard batteries. Without one, the battery cell hides.
- **Webcam**: `scripts/webcam.sh` picks the first USB device with a video interface or "cam" in its product name. To pick a port yourself, set `CAM_USB=<port>` in eww's environment (`grep -il cam /sys/bus/usb/devices/*/product`).
- **Wifi**: `scripts/wifi.sh` reads the active connection from NetworkManager, or the SSID from `iw` without it, whatever the interface is called.
- **Backlight**: `scripts/brightness.sh` uses the first device in `/sys/class/backlight` and its maximum. Without one, the cell stays empty.
- **Monitors**: `scripts/bars.sh` opens a bar on every monitor Hyprland reports, and with `watch` adds or removes bars as monitors are plugged in or out. The power menu opens on the monitor whose bar you clicked; opened from a keybind (`eww open power_menu`), it and `sound` fall back to `:monitor "[1,0]"`, the second monitor if there is one, else the first.
- **Workspace switching**: `scripts/workspace.sh` dispatches the Lua form (`hl.dsp.focus({ workspace = N })`) for a `hyprland.lua` config and falls back to `dispatch workspace N` for `hyprland.conf`.
- **Paths**: everything goes through eww's `EWW_CONFIG_DIR` and `EWW_CMD`, so the bar works from any home folder and any eww binary.

The webcam button turns the camera off by unbinding its USB device, which needs root. It uses `sudo -n`, so add a sudoers rule for exactly those two commands (`sudo visudo -f /etc/sudoers.d/webcam`):

```
<you> ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/bus/usb/drivers/usb/unbind, /usr/bin/tee /sys/bus/usb/drivers/usb/bind
```

Without it the camera indicator still works and the button does nothing.

## Light and dark

`voyager-theme` switches the bar and the [dunst](../dunst/dunst.md) notifications together:

```bash
voyager-theme toggle            # flip light <-> dark
voyager-theme light|dark        # set it
voyager-theme                   # print the current mode
hl.bind("SUPER + T", hl.dsp.exec_cmd("EWW=/path/to/eww " .. os.getenv("HOME") .. "/.local/bin/voyager-theme toggle"))   -- hyprland.lua
```

The mode is saved in `~/.config/voyager/theme` and survives restarts. Switching copies `themes/<mode>.scss` to `~/.config/eww/theme.scss`, which `eww.scss` imports, points the icons at `assets/<mode>/`, and reloads eww. Set `EWW=` if eww is not on `PATH`.

## Contrast

The bar window is transparent, so every cell sits directly on the dark navy wallpaper (`#14141F` → `#05050D`). Each mode keeps its cells apart from it:

- **light**: light mist fill (`rgba(217,217,242,.92)`) with navy `#14141F` text and icons;
- **dark**: navy glass fill (`rgba(10,10,20,.82)`) with mist `#D9D9F2` text and icons, so the frame and bottom edge do the separating;
- every cell has a 1px indigo frame and a 2px bottom edge, so cells sitting next to each other stay separate.

Each mode's colors are only variables, in `eww/themes/light.scss` and `eww/themes/dark.scss`. Keep text and icons in contrast with `$fill`.

## Icons

`eww/icons/light/` holds the navy `#14141F` icons for light mode (battery, brightness, menu-burger, sound-wave, wifi). `eww/icons/dark/` holds the same icons in mist `#D9D9F2`, made with:

```bash
for f in eww/icons/light/*.png; do magick "$f" -fill '#D9D9F2' -colorize 100 "eww/icons/dark/${f##*/}"; done
```

The installer copies them to `~/.config/eww/assets/<mode>/`, where `bimage` in `utils.yuck` loads them from, using the `theme` variable.
