# Plymouth boot screen

An animated screen for unlocking an encrypted disk at boot: the probe in a long-exposure sky of Ursa Major and Minor (with the faint Integrated Flux Nebula, the Owl Nebula and a few galaxies), which reacts as you type.

![preview](../docs/preview.gif)

| When | What happens |
|---|---|
| Waiting | Just the sky. Polaris breathes, and a faint hint says *"type your passphrase, press enter to launch"* |
| You type a character | Stars fly outward in a warp burst, trailing tails that grow with their speed and shrink as they slow; the sky blue-shifts, and a signal ring pulses from the probe's dish |
| Backspace | Stars pull back in, the sky red-shifts, and the ring collapses into the dish |
| Enter | The warp speeds up and the sky turns blue while the disk unlocks |
| Wrong passphrase | The probe flashes red and shakes, the sky red-shifts, the hint comes back |

It never draws bullets, so the screen does not reveal how long your passphrase is. Press **Esc** at boot to get the plain text prompt instead.

Tested on Arch Linux with GRUB, LUKS2, `sd-encrypt` and Intel Iris Xe (i915).

## Prerequisites

This theme needs the **systemd-based initramfs**. The installer checks all of this and tells you what is missing, but never changes it for you.

### 1. systemd initramfs with `sd-encrypt`

```bash
grep '^HOOKS' /etc/mkinitcpio.conf
```

The order must be `systemd` before `plymouth`, and `plymouth` before `sd-encrypt`. For example:

```bash
sudo cp /etc/mkinitcpio.conf /etc/mkinitcpio.conf.bak
sudo sed -i 's|^HOOKS=.*|HOOKS=(base systemd autodetect microcode modconf kms plymouth keyboard sd-vconsole block sd-encrypt filesystems fsck)|' /etc/mkinitcpio.conf
grep '^HOOKS' /etc/mkinitcpio.conf
```

> **Coming from the busybox `encrypt` hook?** Before you reboot, also change the kernel parameter
> `cryptdevice=UUID=<luks-uuid>:cryptroot` to `rd.luks.name=<luks-uuid>=cryptroot`, or the disk will not unlock.
> Using LVM? Keep `lvm2` after `sd-encrypt`. Do not use the old `sd-plymouth` / `plymouth-encrypt` hooks.

### 2. Plymouth and `splash`

```bash
sudo pacman -S plymouth
sudo cp /etc/default/grub /etc/default/grub.bak
sudoedit /etc/default/grub     # add "splash" to GRUB_CMDLINE_LINUX_DEFAULT, e.g. "loglevel=3 quiet splash rd.luks.name=…"
sudo mkinitcpio -P             # stop if you see ==> ERROR
sudo grub-mkconfig -o /boot/grub/grub.cfg
grep -m1 splash /boot/grub/grub.cfg     # must show your linux line
```

## Install

Three steps: copy the theme, try it, then make it the boot theme.

```bash
sudo ./plymouth/install.sh           # 1. copy the theme and check the prerequisites
./plymouth/preview.sh                # 2. try it (from a text console, see below)
sudo ./plymouth/install.sh --apply   # 3. make it the default and rebuild the initramfs
```

The theme is built into the initramfs, so re-run `--apply` after **any** change to it.

### Preview

Switch to a text console (**Ctrl+Alt+F3**, add **Fn** if your F-keys are media keys), log in, and run `./plymouth/preview.sh`:

1. The sky idles for ~3 s.
2. Prompt **one**: type, try Backspace, press Enter, and watch the warp.
3. Prompt **two** acts as if the passphrase was wrong: the probe flashes red and shakes. Type again and press Enter to finish.

Your previous theme comes back when the preview ends. Return to your desktop with **Ctrl+Alt+F1** or **F2**; your session keeps running meanwhile.

### What the scripts change

| Command | Changes | Safety net |
|---|---|---|
| `install.sh` | copies files to `/usr/share/plymouth/themes/voyager/` | only reads `mkinitcpio.conf` and the kernel cmdline |
| `install.sh --apply` | sets `Theme=` in `/etc/plymouth/plymouthd.conf`, rebuilds every kernel's initramfs | copies each `initramfs-<kernel>.img` to `….pre-voyager` first (if `/boot` has room), prints the command to switch back, checks voyager made it in |
| `install.sh --remove` | deletes the theme folder | refuses while voyager is the default |
| `preview.sh` | the default theme, for the preview only, without rebuilding | switches back on exit |

Delete the `.pre-voyager` copies once you have booted fine.

### Remove

```bash
sudo plymouth-set-default-theme -R bgrt     # or your previous theme
sudo ./plymouth/install.sh --remove
```

## Customize

- **Motion:** the tunables at the top of `voyager/voyager.script`, each explained there: the warp stars (how many, their size, brightness and tails), the burst per key, the warp speed, how strongly and how long the sky blue- or red-shifts, and the probe's shake. Values not worth changing are fixed in the code, each with the reason.
- **Text:** search the script for `Image.Text("voyager"` and `Image.Text("type your passphrase`.
- **Images:** every PNG in `voyager/` is rendered by `src/gen-plymouth.py`: the artwork from the SVGs in `src/art/` (and `logo-flash.png`, the probe's red silhouette, from `logo.png`), and the sky (`sky.png`, `polaris.png`, `blueshift.png`, `redshift.png`) from `src/voyager_sky.py`. Edit, run it, then `--apply`. If you move the probe's dish, update `dish_x` / `dish_y` in the script.
- **Screen size:** the sky is 1920×1080 and is scaled to cover the screen. For a sharper one on a bigger screen: `src/gen-plymouth.py --size auto` (or `--size 2560x1440`), then `--apply`. The sky adds about 2 MB to the initramfs.
- **Shift strength:** `SHIFT_FRACTION`, `BLUE_SHIFT`, `RED_SHIFT` and `SHIFT_SAT` in `src/voyager_sky.py`.
- **Fallback background:** `Window.SetBackgroundTopColor` / `BottomColor` in the script (RGB, 0–1), seen only if `sky.png` is missing.

## If it breaks

Start with `./doctor.sh plymouth` (no sudo needed).

| Symptom | Do this |
|---|---|
| Plain text prompt instead of the theme | check [prerequisites](#prerequisites): `splash` on the cmdline, hook order |
| Old theme still shows | `sudo ./plymouth/install.sh --apply` |
| Splash appears late or flickers (Intel) | set `MODULES=(i915)` in `/etc/mkinitcpio.conf`, then `sudo mkinitcpio -P` |
| Theme is broken and you want the old one back | `sudo plymouth-set-default-theme -R <previous theme>` (`--apply` printed it) |
| Disk does not unlock after switching hooks | at the GRUB menu press `e` and fix `rd.luks.name=…` on the linux line |
| Boot fails after `--apply` | at the GRUB menu press `e` and point the `initrd` line at the `….pre-voyager` image |
| Preview hangs | Ctrl+C, then `sudo plymouth quit \|\| sudo pkill plymouthd` |
