# voyager — eww bar

A complete [eww](https://github.com/elkowar/eww) top bar, on every monitor, in the voyager palette: light mist cells with navy text and icons, indigo edges, teal for "ok" states (wifi connected, charging, camera on), amber for mid battery, and the hyprlock fail red for low battery or no wifi.

Left: power menu, clock, date, Hyprland workspaces (click one to switch to it; each bar highlights the workspace on its own monitor). Right: wifi, volume slider, mixer (pavucontrol), brightness, webcam toggle, battery (tooltip: time left), system tray.

## Install

```bash
sudo pacman -S jq socat inotify-tools libpulse wireplumber pavucontrol   # what the bar's scripts call
                                                                          # + networkmanager or iw, for wifi
./eww/install.sh             # as your user, not sudo
```

`eww` itself must be on `PATH`. Arch ships it in the AUR (`eww`), or build it from source.

The installer copies the whole bar into `~/.config/eww`: `eww.yuck`, `var.yuck`, `utils.yuck`, `audio/`, `scripts/`, `eww.scss`, and the icons into `assets/`. It reports any missing packages and reloads eww if it is running.

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

## Contrast

The bar window is transparent, so every cell sits directly on the dark navy wallpaper (`#14141F` → `#05050D`). Cells need to stand out from it:

- the cell fill is light mist (`rgba(217,217,242,.92)`), far brighter than the wallpaper;
- text and icons are navy `#14141F`, so they read on the light fill;
- every cell has a 1px indigo border, so cells sitting next to each other stay separate.

If you change the palette, keep the cell fill light. If you switch to dark cells, also recolor `eww/icons/` to a light color, or they will disappear.

## Icons

`eww/icons/` holds the icons the bar uses (battery, brightness, menu-burger, sound-wave, wifi), colored navy `#14141F` so they read on the light cells. The installer copies them into `~/.config/eww/assets/`, where `bimage` in `utils.yuck` loads them from.
