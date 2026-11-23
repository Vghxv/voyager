# voyager — hyprlock

A static companion for the lock screen: same starfield, probe and palette as the boot screen.

![hyprlock](../docs/hyprlock.png)

| Plymouth | hyprlock |
|---|---|
| Stars twinkle smoothly | Starfield **crossfades between 6 frames, one per second** (`reload_cmd` + `fadeIn` animation) |
| Warp + signal rings on typing | Not possible — hyprlock has no scripting. Typed characters appear as **✦ stars that fade in** |
| Red flash + shake on wrong passphrase | Field outline turns red · *"signal lost — N"* |
| Hint text | Placeholder *"password, then enter to launch"* |

## Install

```bash
./hyprlock/install.sh          # as your user, not sudo
hyprlock                       # test: unlock with your user password
```

- Assets go to `~/.config/hypr/voyager/`, the config to `~/.config/hypr/hyprlock.conf`.
- Your current `hyprlock.conf` is backed up to `hyprlock.conf.bak-<date>`. If it already is the voyager config, nothing is backed up, so re-running never hides your original.
- Undo: `./hyprlock/install.sh --restore` restores the newest backup. The assets in `~/.config/hypr/voyager/` stay.

## Notes

- Every monitor shows the full lock screen (starfield, probe, title, clock, password field). To limit the widgets to one screen, set `monitor = <name>` in each widget block of `hyprlock/hyprlock.conf` (`hyprctl monitors | grep ^Monitor`).
- The frames are rendered at 1920×1080. The installer warns when a monitor is bigger; `src/gen-hyprlock.py --size auto` re-renders them for your largest monitor.
- Re-render the frames and probe with `src/gen-hyprlock.py` (`--size`, `--seed`), then run `./hyprlock/install.sh`. It reuses the wallpaper's gradient and stars, so both use the same palette. It writes 6 frames, matching `next-frame.sh`.
- If `✦` renders as a box, your font lacks the glyph: set `dots_text_format = *` or install `noto-fonts`.

> On Arch, hyprlock uses `pam_faillock`: **3 wrong passwords lock you out for 10 minutes** (`/etc/security/faillock.conf`).
>
> If hyprlock ever crashes into a red screen, switch to a TTY and run:
> `hyprctl --instance 0 'keyword misc:allow_session_lock_restore 1'` then `hyprctl --instance 0 dispatch exec hyprlock`.
