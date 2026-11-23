# voyager — dunst

Notifications styled like the eww bar's HUD cells: square panels (no rounded corners), monospace text, a 1px frame and a `▸` marker on the summary, dropping in top-right just below the bar. They switch between light and dark together with the bar: `~/.config/scripts/voyager-theme toggle` (see [eww.md](../eww/eww.md#light-and-dark)).

| Urgency | Light | Dark | Timeout |
|---|---|---|---|
| low | mist panel, dim text, faint indigo frame | navy glass, dim text, faint indigo frame | 5 s |
| normal | mist panel, navy text, indigo frame | navy glass, mist text, indigo frame | 8 s |
| critical | pink tint, red `#FF3355` frame | dark red tint, pink text, red frame | stays until clicked |

The progress bar is teal (red on critical).

## Install

```bash
./dunst/install.sh             # as your user, not sudo
notify-send -u critical voyager 'signal lost'
```

- The config goes to `~/.config/dunst/dunstrc`. It has no colors: the saved mode's colors go to `~/.config/dunst/dunstrc.d/90-voyager-theme.conf` (from `dunst/themes/light.conf` or `dark.conf`), which dunst reads after it. A running dunst is reloaded (`dunstctl reload`).
- The installer also installs `~/.config/scripts/voyager-theme`, shared with the eww bar.
- Your current `dunstrc` is backed up to `dunstrc.bak-<date>`. If it already is the voyager config, nothing is backed up.
- Undo: `./dunst/install.sh --restore` removes the color drop-in and restores the newest backup. With no backup (dunst was using `/etc/dunst/dunstrc`), just delete `~/.config/dunst/dunstrc` and run `dunstctl reload`.

## Notes

- Clicks: left runs the default action and closes, middle closes, right closes all.
- Progress bars: `notify-send -h int:value:60 …`.
- Notifications follow the monitor with the mouse. `offset = (10, 50)` clears the bar; adjust it if your bar is taller.
- The font is Geist Mono Nerd Font, falling back to JetBrains Mono, then any monospace font.
- Icons come from the Adwaita theme; apps that send their own icon keep it.
