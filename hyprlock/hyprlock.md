# hyprlock lock screen

The lock screen: the wallpaper's sky (Ursa Major and Minor, the faint Integrated Flux Nebula, the Owl Nebula and a few galaxies; see [the wallpaper](../awww/awww.md)) with the boot screen's probe and colors on top.

hyprlock cannot run animations the way Plymouth does, so the effects are adapted:

| Plymouth (boot) | hyprlock (lock screen) |
|---|---|
| Stars twinkle smoothly | Polaris and a few faint stars twinkle: hyprlock shows one still image, so `next-frame.sh` hands it the next of six frames every second and it crossfades |
| Stars warp and rings pulse as you type | Each typed character appears as a ✦ star that fades in |
| Red flash and shake on a wrong passphrase | The field outline turns red: *"signal lost — N"* |

## Install

```bash
./hyprlock/install.sh
hyprlock                 # try it: unlock with your user password
```

Your current `~/.config/hypr/hyprlock.conf` is backed up first. `./hyprlock/install.sh --restore` brings it back. The images in `~/.config/hypr/voyager/` stay; they are shared with the wallpaper.

## Adjust

- **One monitor only:** every monitor shows the full lock screen. To show the widgets on one screen, set `monitor = <name>` in each widget block of `hyprlock.conf` (names: `hyprctl monitors | grep ^Monitor`).
- **Bigger screen:** the frames are 1920×1080, and the installer warns if your monitor is bigger. Re-render them for it:

  ```bash
  src/gen-hyprlock.py --size auto      # other options: --help
  ./hyprlock/install.sh
  ```

## If it breaks

Start with `./doctor.sh hyprlock`.

| Symptom | Do this |
|---|---|
| Password dots show as boxes | your font lacks `✦`: install `noto-fonts`, or set `dots_text_format = *` |
| Background is black or frozen | `./hyprlock/install.sh` |
| Locked out after wrong passwords | wait 10 minutes: Arch locks you out after 3 failed attempts (`/etc/security/faillock.conf`) |
| hyprlock crashed into a red screen | switch to a TTY (Ctrl+Alt+F3), log in, then run the two commands below |

```bash
hyprctl --instance 0 'keyword misc:allow_session_lock_restore 1'
hyprctl --instance 0 dispatch exec hyprlock
```
