# voyager — Plymouth

An animated LUKS unlock screen: a deep-space probe drifting in a twinkling starfield that reacts to your typing.

![preview](../docs/preview.gif)

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
sudo ./plymouth/install.sh          # copies the theme, checks prerequisites
```

### Preview without rebooting

Switch to a TTY (**Ctrl+Alt+F3**), log in, then:

```bash
./plymouth/preview.sh
```

1. Idle starfield for ~3 s
2. Prompt **one** — type, try Backspace, press Enter → warp
3. Prompt **two** — the theme treats this as a wrong passphrase → red flash + shake; type again, Enter → warp, quit

The previous default theme is restored when the preview ends.
Back to your desktop: **Ctrl+Alt+F1** or **F2**. Add **Fn** if your F-keys are media keys. Switching consoles does not stop the desktop session.

### Apply

```bash
sudo ./plymouth/install.sh --apply
lsinitcpio /boot/initramfs-linux.img | grep themes/voyager     # expect 7 files
```

`--apply` prints your previous theme and the command to switch back. Before rebuilding, it copies the current image to `/boot/initramfs-linux.img.pre-voyager` if `/boot` has room. Delete that copy once you have booted fine.

The theme lives inside the initramfs — re-run `sudo ./plymouth/install.sh --apply` after **any** edit.

### Remove

```bash
sudo plymouth-set-default-theme -R bgrt
sudo ./plymouth/install.sh --remove
```

### What the scripts change

| Script | Touches | Safety |
|---|---|---|
| `install.sh` | `/usr/share/plymouth/themes/voyager/` | only checks `mkinitcpio.conf` / cmdline, never edits them |
| `install.sh --apply` | `Theme=` in `/etc/plymouth/plymouthd.conf`, rebuilds all initramfs images | backs up `initramfs-linux.img` → `.pre-voyager` · prints revert command |
| `install.sh --remove` | deletes the theme dir | refuses while voyager is the default |
| `preview.sh` | default theme, temporarily (no rebuild) | restored on exit |

### Manual install

```bash
sudo mkdir -p /usr/share/plymouth/themes/voyager
sudo cp plymouth/voyager/* /usr/share/plymouth/themes/voyager/
sudo plymouth-set-default-theme -R voyager
```

---

## Customize

Tunables at the top of `plymouth/voyager/voyager.script`:

| Variable | Default | Effect |
|---|---|---|
| `NUM_STARS` | `110` | number of stars |
| `KEY_BOOST` | `1.4` | warp burst per typed character |
| `BACK_PULL` | `1.0` | reverse pull per backspace |
| `WARP_MAX` | `6` | warp speed after Enter |
| `STREAK_AT` | `1.2` | speed at which stars become streaks |
| `ROT_SIGN` | `1` | set to `-1` if streaks point sideways instead of outward |

Other edits:

- **Logo** — replace `plymouth/voyager/logo.png` (transparent PNG, ~400 px). If you change the artwork, update the dish position in the script (`dish_x` / `dish_y`, coordinates in the 400×400 logo) so the rings start at the right spot.
- **Title / hint text** — `Image.Text("voyager", …)` and `Image.Text("type your passphrase…", …)`.
- **Colors** — `Window.SetBackgroundTopColor` / `BottomColor` (RGB 0–1); ring and streak colors live in `src/*.svg`. Re-render with `rsvg-convert -w <W> -h <H> src/ring.svg -o plymouth/voyager/ring.png`.
