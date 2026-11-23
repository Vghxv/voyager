# voyager — eww bar

Restyles an existing [eww](https://github.com/elkowar/eww) bar with the voyager palette: light mist cells with navy text and icons, indigo edges, teal for "ok" states (wifi connected, charging, camera on), amber for mid battery, and the hyprlock fail red for low battery or no wifi.

Only `eww.scss` and the icons change. The class names match the original bar, so your `.yuck` files keep working as they are.

## Install

```bash
./eww/install.sh             # as your user, not sudo
```

Deployment is one way: repo → `~/.config/eww`. The installer:

- copies `eww/eww.scss` over `~/.config/eww/eww.scss`;
- copies `eww/icons/*.png` into `~/.config/eww/assets/`;
- runs `eww reload` if the daemon is running.

It never reads anything from `~/.config/eww` and makes no backups. This repo is the source of truth: edit the files here and run the installer again. Your `.yuck` files are not touched.

## Contrast

The bar window is transparent, so every cell sits directly on the dark navy wallpaper (`#14141F` → `#05050D`). Cells need to stand out from it:

- the cell fill is light mist (`rgba(217,217,242,.92)`), far brighter than the wallpaper;
- text and icons are navy `#14141F`, so they read on the light fill;
- every cell has a 1px indigo border, so cells sitting next to each other stay separate.

If you change the palette, keep the cell fill light. If you switch to dark cells, also recolor the icons to a light color (see below), or they will disappear.

## Icons

`eww/assets/` holds the original black icons. `eww/icons/` holds the copies that actually get installed, recolored to navy. To regenerate them (needs ImageMagick):

```bash
eww/recolor.sh               # navy #14141F
eww/recolor.sh '#D9D9F2'     # any other color, e.g. mist for a dark bar
./eww/install.sh
```
