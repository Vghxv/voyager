# eww bar

A top bar on every monitor, styled as a HUD: square cells, monospace text, a thin indigo frame. Teal means "ok" (wifi connected, charging, camera on), amber means a half-empty battery, red means low battery or no wifi. It has a [light and a dark mode](../README.md#light-and-dark).

- **Left:** power menu, clock, date, workspaces. Click a workspace to switch to it; each bar highlights the workspace on its own monitor.
- **Right:** wifi, volume slider, [volume button](#volume), brightness, webcam on/off, battery (hover for time left), system tray.

A click outside the power menu or volume popup closes it.

## Install

```bash
./eww/install.sh
```

`eww` must be on `PATH` (AUR: `eww`, or build it). The installer lists any other packages the bar needs and copies everything to `~/.config/eww`. Running it again overwrites voyager's files there, leaves your other files alone and restarts a running bar. There is no undo: edit the files in this repo, not in `~/.config/eww`, and re-run the installer.

## Start

```bash
~/.config/eww/scripts/bars.sh           # open a bar on every monitor now
~/.config/eww/scripts/bars.sh toggle    # hide / show all bars (for a keybind)
```

To start it at login and follow monitors being plugged in or out, add `bars.sh watch`:

```ini
exec-once = ~/.config/eww/scripts/bars.sh watch        # hyprland.conf
```

```lua
hl.exec_cmd(os.getenv("HOME") .. "/.config/eww/scripts/bars.sh watch")   -- hyprland.lua, in hyprland.start
```

Use `bars.sh` rather than `eww open bar`: each bar needs to be told its monitor. If eww is not on `PATH`, prefix the command with `EWW=/path/to/eww`.

## Detected automatically

Nothing needs editing per machine.

- **Battery:** the laptop's own battery, not a mouse or keyboard battery. No battery, no battery cell.
- **Backlight:** the first one found. None, and the brightness cell stays empty.
- **Wifi:** NetworkManager, or `iw` without it. Any interface name.
- **Monitors:** a bar on every monitor. The power menu opens on the monitor whose bar you clicked.
- **Hyprland config:** workspace switching works with both `hyprland.lua` and `hyprland.conf`.
- **Webcam:** the first USB camera. To choose one, set `CAM_USB=<port>` in eww's environment (find it with `grep -il cam /sys/bus/usb/devices/*/product`).

## Webcam

The webcam button turns the camera off by disconnecting its USB device, which needs root. Allow exactly that without a password (`sudo visudo -f /etc/sudoers.d/webcam`):

```
<you> ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/bus/usb/drivers/usb/unbind, /usr/bin/tee /sys/bus/usb/drivers/usb/bind
```

Without this rule the camera indicator still works, but the button does nothing.

## Volume

The speaker-wave button: left click opens a speaker + mic popup under it, right click opens pavucontrol. The bar and popup control whichever device is the default, and follow it when it switches (e.g. headphones connecting).

Each scroll notch moves the sliders by `volume_scroll_amount` (in `var.yuck`, in percent) through `:scroll-amount`, which stock eww 0.6 does not have: it needs an eww built with that addition to its `scale` widget. With stock eww the sliders move 1 point per notch and `eww logs` warns that `scroll-amount` is an unknown attribute; everything else works.

## Change the look

- **Colors:** each mode is one map in `$modes` at the top of `eww.scss`. That file's header explains how the modes and contrast work.
- **Icons:** `icons/light/` holds the dark-ink icons used in light mode. Make the light-ink set for dark mode from them:

  ```bash
  for f in eww/icons/light/*.png; do magick "$f" -fill '#D9D9F2' -colorize 100 "eww/icons/dark/${f##*/}"; done
  ```

Then run `./eww/install.sh`.

## If it breaks

Start with `./doctor.sh eww`. Every bar script writes the reason it cannot work to `eww logs`.

| Symptom | Do this |
|---|---|
| No bar at all | `~/.config/eww/scripts/bars.sh`, then read its error, or `eww logs` |
| Bar on one monitor only | `bars.sh` again; for hotplug, start it with `bars.sh watch` |
| A cell is empty (workspaces, volume, brightness) | `eww logs`: it names the missing package or device |
| Bar looks wrong after a `git pull` | `./eww/install.sh` |
| Webcam button does nothing | add the sudoers rule above |
| Light/dark switch does not change the bar | `~/.config/scripts/voyager-theme apply` shows the error |
