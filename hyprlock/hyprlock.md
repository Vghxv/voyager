# hyprlock lock screen

The lock screen: the same view of the sky as your wallpaper (see [the wallpaper](../awww/awww.md)), without the wallpaper's small probe, with the boot screen's big probe and colors on top. When you pick a new view in drag mode, the lock screen follows it.

hyprlock cannot run animations the way Plymouth does, so the effects are adapted:

| Plymouth (boot) | hyprlock (lock screen) |
|---|---|
| Polaris breathes smoothly | The sky is still: it's `lock.png`, written by voyager-sky with the wallpaper |
| Stars warp, the sky blue-shifts and rings pulse as you type | Each typed character appears as a ✦ star that fades in |
| Red flash and shake on a wrong passphrase | The field outline turns red: *"signal lost — N"* |

## Install

```bash
./hyprlock/install.sh
hyprlock                 # try it: unlock with your user password
```

Install [the wallpaper](../awww/awww.md) first: the lock screen's background is its `~/.config/hypr/voyager/lock.png`.

Your current `~/.config/hypr/hyprlock.conf` is backed up first. `./hyprlock/install.sh --restore` brings it back. The images in `~/.config/hypr/voyager/` stay (the wallpaper keeps its own files in that folder too).

## Adjust

- **One monitor only:** every monitor shows the full lock screen. To show the widgets on one screen, set `monitor = <name>` in each widget block of `hyprlock.conf` (names: `hyprctl monitors | grep ^Monitor`).
- **Bigger screen:** `lock.png` is rendered at your largest monitor's size. After plugging in a bigger one, run `voyager-sky apply`.
- **The probe image:** `probe.png` is `src/logo.svg` at 400 px. After changing the logo: `rsvg-convert -w 400 -h 400 src/logo.svg -o hyprlock/probe.png`.

## If it breaks

Start with `./doctor.sh hyprlock`.

| Symptom | Do this |
|---|---|
| Password dots show as boxes | your font lacks `✦`: install `noto-fonts`, or set `dots_text_format = *` |
| Background is black | the wallpaper's `lock.png` is missing: `./awww/install.sh` |
| Locked out after wrong passwords | wait 10 minutes: Arch locks you out after 3 failed attempts (`/etc/security/faillock.conf`) |
| hyprlock crashed into a red screen | switch to a TTY (Ctrl+Alt+F3), log in, then run the two commands below |

```bash
hyprctl --instance 0 'keyword misc:allow_session_lock_restore 1'
hyprctl --instance 0 dispatch exec hyprlock
```
