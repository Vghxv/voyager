#!/usr/bin/env bash
# Webcam on/off by binding/unbinding its USB device.
#
#   webcam.sh status   prints On / Off (the bar polls this)
#   webcam.sh toggle   bind <-> unbind
#
# The webcam's USB port is detected: the first USB device with a video interface
# (class 0e) or "cam" in its product name. The name check finds it while it is
# unbound too, since unbinding removes the interfaces but not the device.
# Set CAM_USB (e.g. CAM_USB=1-3) to pick a port yourself.
# toggle needs passwordless sudo for exactly these two commands (sudoers):
#   <you> ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/bus/usb/drivers/usb/unbind, /usr/bin/tee /sys/bus/usb/drivers/usb/bind

DRIVER=/sys/bus/usb/drivers/usb

find_webcam() {
  local d
  for d in /sys/bus/usb/devices/*/; do
    d=${d%/}
    [[ -f "$d/product" ]] || continue
    if grep -qx 0e "$d"/*:*/bInterfaceClass 2>/dev/null || grep -qi cam "$d/product"; then
      basename "$d"; return
    fi
  done
}

DEVICE_ID="${CAM_USB:-$(find_webcam)}"

case "${1:-status}" in
  status)
    [[ -n "$DEVICE_ID" && -e "$DRIVER/$DEVICE_ID" ]] && echo On || echo Off ;;
  toggle)
    [[ -n "$DEVICE_ID" ]] || { echo "webcam.sh: no webcam found (set CAM_USB)" >&2; exit 1; }
    if [[ -e "$DRIVER/$DEVICE_ID" ]]; then op=unbind; else op=bind; fi
    echo "$DEVICE_ID" | sudo -n /usr/bin/tee "$DRIVER/$op" >/dev/null ;;
  *)
    echo "usage: $0 status|toggle" >&2; exit 1 ;;
esac
