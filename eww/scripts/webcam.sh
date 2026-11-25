#!/usr/bin/env bash
# Webcam on/off by binding/unbinding its USB device.
#
#   webcam.sh status   prints On / Off (the bar polls this)
#   webcam.sh toggle   bind <-> unbind, then show the new state in the bar right away
#                      (the poll only runs every few seconds)
#
# The webcam's USB port is detected: the first USB device with a video interface
# (class 0e) or "cam" in its product name. The name check finds it while it is
# unbound too, since unbinding removes the interfaces but not the device.
# Set CAM_USB (e.g. CAM_USB=1-3) to pick a port yourself.
#
# Needs: a sudoers rule for toggle — see eww/eww.md#webcam

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

status() { if [[ -n "$DEVICE_ID" && -e "$DRIVER/$DEVICE_ID" ]]; then echo On; else echo Off; fi; }

case "${1:-status}" in
  status) status ;;
  toggle)
    [[ -n "$DEVICE_ID" ]] || { echo "webcam.sh: no webcam found (set CAM_USB)" >&2; exit 1; }
    if [[ -e "$DRIVER/$DEVICE_ID" ]]; then op=unbind; else op=bind; fi
    echo "$DEVICE_ID" | sudo -n /usr/bin/tee "$DRIVER/$op" >/dev/null ||
      { echo "webcam.sh: sudo refused — add the sudoers rule from eww/eww.md#webcam" >&2; exit 1; }
    "${EWW:-eww}" update camera="$(status)" 2>/dev/null || true ;;
  *)
    echo "usage: $0 status|toggle" >&2; exit 1 ;;
esac
