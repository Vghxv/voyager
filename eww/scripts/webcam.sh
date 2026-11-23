#!/usr/bin/env bash
# Webcam on/off by binding/unbinding its USB device.
#
#   webcam.sh status   prints On / Off (the bar polls this)
#   webcam.sh toggle   bind <-> unbind
#
# Set DEVICE_ID to your webcam's USB port: grep -il camera /sys/bus/usb/devices/*/product
# toggle needs passwordless sudo for exactly these two commands (sudoers):
#   <you> ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/bus/usb/drivers/usb/unbind, /usr/bin/tee /sys/bus/usb/drivers/usb/bind

DEVICE_ID="${CAM_USB:-1-3}"
DRIVER=/sys/bus/usb/drivers/usb

case "${1:-status}" in
  status)
    [[ -e "$DRIVER/$DEVICE_ID" ]] && echo On || echo Off ;;
  toggle)
    if [[ -e "$DRIVER/$DEVICE_ID" ]]; then op=unbind; else op=bind; fi
    echo "$DEVICE_ID" | sudo -n /usr/bin/tee "$DRIVER/$op" >/dev/null ;;
  *)
    echo "usage: $0 status|toggle" >&2; exit 1 ;;
esac
