# dunst notifications

Notifications that match the eww bar: square panels with a thin frame, monospace text and a `▸` before the title. They appear top-right, just below the bar, and switch between [light and dark](../README.md#light-and-dark) together with it.

| Urgency | Frame | Disappears after |
|---|---|---|
| low | faint indigo, dimmed text | 5 s |
| normal | indigo | 8 s |
| critical | red | stays until clicked |

Progress bars (`notify-send -h int:value:60 …`) are teal, or red on critical.

## Install

```bash
./dunst/install.sh
notify-send -u critical voyager 'signal lost'      # try it
```

Your current `~/.config/dunst/dunstrc` is backed up first. `./dunst/install.sh --restore` brings it back and removes voyager's colors. If you had no `dunstrc` of your own, there is nothing to restore: delete `~/.config/dunst/dunstrc` and run `dunstctl reload`.

## Using it

- **Clicks:** left runs the notification's action and closes it, middle closes it, right closes all.
- **Position:** notifications follow the monitor your mouse is on. If your bar is taller, raise the `50` in `offset = (10, 50)` in `dunstrc`.
- **Font:** Geist Mono Nerd Font, else JetBrains Mono, else any monospace font.
- **Icons:** from the Adwaita theme. Apps that send their own icon keep it.

To change colors, edit `themes/light.conf` or `themes/dark.conf` and re-run the installer. Layout and behaviour are in `dunstrc`.

## If it breaks

Start with `./doctor.sh dunst`.

| Symptom | Do this |
|---|---|
| Notifications look like plain dunst | `./dunst/install.sh` |
| Wrong colors for the current mode | `~/.config/scripts/voyager-theme apply` |
| No notifications at all | `pkill -x dunst`, then `notify-send test`: dunst restarts on its own |
| Notifications hide behind or overlap the bar | raise the `50` in `offset` in `dunstrc` |
