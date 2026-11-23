# voyager — dunst

Notifications styled like the eww bar's cells: light mist panels, dark navy text, a 1px indigo edge and 8px corners, dropping in top-right just below the bar.

| Urgency | Look | Timeout |
|---|---|---|
| low | dim text, faint indigo edge | 5 s |
| normal | navy text, indigo edge, teal progress bar | 8 s |
| critical | pink tint, red `#FF3355` edge (hyprlock's fail color) | stays until clicked |

## Install

```bash
./dunst/install.sh             # as your user, not sudo
notify-send -u critical voyager 'signal lost'
```

- The config goes to `~/.config/dunst/dunstrc`, and a running dunst is reloaded (`dunstctl reload`).
- Your current `dunstrc` is backed up to `dunstrc.bak-<date>`. If it already is the voyager config, nothing is backed up.
- Undo: `./dunst/install.sh --restore` restores the newest backup. With no backup (dunst was using `/etc/dunst/dunstrc`), just delete `~/.config/dunst/dunstrc` and run `dunstctl reload`.

## Notes

- Clicks: left runs the default action and closes, middle closes, right closes all.
- Progress bars: `notify-send -h int:value:60 …` draws a teal bar (red on critical).
- Notifications follow the monitor with the mouse. `offset = (10, 50)` clears the bar; adjust it if your bar is taller.
- Icons come from the Adwaita theme; apps that send their own icon keep it.
