# hyprlock lock screen

The lock screen: a still sky with the big probe and the voyager colors on top. Each typed character appears as a ✦ star that fades in. A wrong password turns the field outline red: *"signal lost — N"*.

The sky is whatever image is at `~/.config/hypr/voyager/lock.png`, shown as it is. Without one, the background is plain navy.

## Install

```bash
./hyprlock/install.sh
hyprlock                 # try it: unlock with your user password
```

Your current `~/.config/hypr/hyprlock.conf` is backed up first. `./hyprlock/install.sh --restore` brings it back. The probe image stays in `~/.config/hypr/voyager/`.

## Adjust

- **One monitor only:** every monitor shows the full lock screen. To show the widgets on one screen, set `monitor = <name>` in each widget block of `hyprlock.conf` (names: `hyprctl monitors | grep ^Monitor`).
- **The probe image:** `probe.png` is `src/art/logo.svg` at 400 px. After changing the logo: `rsvg-convert -w 400 -h 400 src/art/logo.svg -o hyprlock/probe.png`.

## If it breaks

Start with `./doctor.sh hyprlock`.

| Symptom | Do this |
|---|---|
| Password dots show as boxes | your font lacks `✦`: install `noto-fonts`, or set `dots_text_format = *` |
| Background is plain navy | there is no `~/.config/hypr/voyager/lock.png` |
| Locked out after wrong passwords | wait 10 minutes: Arch locks you out after 3 failed attempts (`/etc/security/faillock.conf`) |
| hyprlock crashed into a red screen | switch to a TTY (Ctrl+Alt+F3), log in, then run the two commands below |

```bash
hyprctl --instance 0 'keyword misc:allow_session_lock_restore 1'
hyprctl --instance 0 dispatch exec hyprlock
```
