# voyager — Plymouth boot theme

An animated LUKS unlock screen for Plymouth: a deep-space probe drifting in a twinkling starfield that reacts to your typing.

![preview](docs/preview.gif)

| Event | Effect |
|---|---|
| Waiting | ~110 stars twinkle, each with its own speed and brightness · faint hint *"type your passphrase, press enter to launch"* |
| Each character | **Warp burst** — stars streak outward · **signal ring** pulses from the probe's dish |
| Backspace | Stars pulled back inward · ring collapses into the dish |
| Enter | Warp accelerates to full speed while the disk unlocks |
| Wrong passphrase | Red flash · probe shakes · hint returns |

No bullets are drawn, so the screen never reveals the passphrase length. Press **Esc** at boot to fall back to the plain text prompt.

Tested on Arch Linux · GRUB · LUKS2 · `sd-encrypt` · Intel Iris Xe (i915).

---

## Repository layout

```
theme/voyager/      the installable theme (script, .plymouth, 6 PNGs)
src/                SVG sources for the images
docs/               preview images
install.sh          copy / apply / remove
preview.sh          test the theme from a TTY without rebooting
hyprlock/           matching hyprlock theme (config, 6 starfield frames, probe)
install-hyprlock.sh install / restore the hyprlock theme
```

---

## Prerequisites

The theme uses Plymouth's password prompt, which works with the **systemd-based initramfs**.

### 1. systemd initramfs with `sd-encrypt`

```bash
grep '^HOOKS' /etc/mkinitcpio.conf
```

Required order: `systemd` before `plymouth`, `plymouth` before `sd-encrypt`. Example:

```bash
sudo cp /etc/mkinitcpio.conf /etc/mkinitcpio.conf.bak
sudo sed -i 's|^HOOKS=.*|HOOKS=(base systemd autodetect microcode modconf kms plymouth keyboard sd-vconsole block sd-encrypt filesystems fsck)|' /etc/mkinitcpio.conf
grep '^HOOKS' /etc/mkinitcpio.conf
```

> Coming from the busybox `encrypt` hook? Also switch the kernel parameter
> `cryptdevice=UUID=<luks-uuid>:cryptroot` → `rd.luks.name=<luks-uuid>=cryptroot`
> **before** rebooting, or the disk will not unlock. Use LVM? Keep `lvm2` after `sd-encrypt`.
> Do not use the removed `sd-plymouth` / `plymouth-encrypt` hooks.

### 2. Plymouth + `splash`

```bash
sudo pacman -S plymouth
sudo cp /etc/default/grub /etc/default/grub.bak
# add "splash" to GRUB_CMDLINE_LINUX_DEFAULT, e.g. "loglevel=3 quiet splash rd.luks.name=…"
sudoedit /etc/default/grub
grep CMDLINE_LINUX_DEFAULT /etc/default/grub
sudo mkinitcpio -P                                   # stop on ==> ERROR
sudo grub-mkconfig -o /boot/grub/grub.cfg
grep -m1 splash /boot/grub/grub.cfg                  # must show your linux line
```

Late or flickering splash on Intel: set `MODULES=(i915)` in `/etc/mkinitcpio.conf` and rebuild.

---

## Install

```bash
git clone <this-repo> voyager-plymouth && cd voyager-plymouth
sudo ./install.sh          # copies theme, checks prerequisites
```

### Preview without rebooting

Switch to a TTY(**Ctrl+Alt+F3**), log in, then:

```bash
./preview.sh
```

1. Idle starfield for ~3 s
2. Prompt **one** — type, try Backspace, press Enter → warp
3. Prompt **two** — the theme treats this as a wrong passphrase → red flash + shake; type again, Enter → warp, quit

Back to your desktop: **Ctrl+Alt+F1** or **F2**. Add **Fn** if your F-keys are media keys. Switching consoles does not stop the desktop session.

### Apply

```bash
sudo ./install.sh --apply
lsinitcpio /boot/initramfs-linux.img | grep themes/voyager     # expect 7 files
```

The theme lives inside the initramfs — re-run `sudo ./install.sh --apply` after **any** edit.

### Remove

```bash
sudo plymouth-set-default-theme -R bgrt
sudo ./install.sh --remove
```

---

## Manual install

```bash
sudo mkdir -p /usr/share/plymouth/themes/voyager
sudo cp theme/voyager/* /usr/share/plymouth/themes/voyager/
sudo plymouth-set-default-theme -R voyager
```

---

## Matching hyprlock theme

A static companion for the lock screen: same starfield, probe and palette.

![hyprlock](docs/hyprlock.png)

| Plymouth | hyprlock |
|---|---|
| Stars twinkle smoothly | Starfield **crossfades between 6 frames, one per second** (`reload_cmd` + `fadeIn` animation) |
| Warp + signal rings on typing | Not possible — hyprlock has no scripting. Typed characters appear as **✦ stars that fade in** |
| Red flash + shake on wrong passphrase | Field outline turns red · *"signal lost — N"* |
| Hint text | Placeholder *"password, then enter to launch"* |

Every monitor shows the full lock screen (starfield, probe, title, clock, password field). To limit the widgets to one screen, set `monitor = <name>` in each widget block of `hyprlock/hyprlock.conf` (`hyprctl monitors | grep ^Monitor`). The frames are rendered at 1920×1080.

### Install

```bash
./install-hyprlock.sh          # as your user, not sudo — backs up the current hyprlock.conf
hyprlock                       # test: unlock with your user password
```

Assets go to `~/.config/hypr/voyager/`, the config to `~/.config/hypr/hyprlock.conf`.

Undo: `./install-hyprlock.sh --restore`

> On Arch, hyprlock uses `pam_faillock`: **3 wrong passwords lock you out for 10 minutes** (`/etc/security/faillock.conf`).
>
> If hyprlock ever crashes into a red screen, switch to a TTY and run:
> `hyprctl --instance 0 'keyword misc:allow_session_lock_restore 1'` then `hyprctl --instance 0 dispatch exec hyprlock`.

If `✦` renders as a box, your font lacks the glyph: set `dots_text_format = *` or install `noto-fonts`.

---

## Customize

Tunables at the top of `theme/voyager/voyager.script`:

| Variable | Default | Effect |
|---|---|---|
| `NUM_STARS` | `110` | number of stars |
| `KEY_BOOST` | `1.4` | warp burst per typed character |
| `BACK_PULL` | `1.0` | reverse pull per backspace |
| `WARP_MAX` | `6` | warp speed after Enter |
| `STREAK_AT` | `1.2` | speed at which stars become streaks |
| `ROT_SIGN` | `1` | set to `-1` if streaks point sideways instead of outward |

Other edits:

- **Logo** — replace `theme/voyager/logo.png` (transparent PNG, ~400 px). If you change the artwork, update the dish position in the script (`dish_x` / `dish_y`, coordinates in the 400×400 logo) so the rings start at the right spot.
- **Title / hint text** — `Image.Text("voyager", …)` and `Image.Text("type your passphrase…", …)`.
- **Colors** — `Window.SetBackgroundTopColor` / `BottomColor` (RGB 0–1); ring and streak colors live in `src/*.svg`. Re-render with `rsvg-convert -w <W> -h <H> src/ring.svg -o theme/voyager/ring.png`.

---
## License

[MIT](LICENSE)
