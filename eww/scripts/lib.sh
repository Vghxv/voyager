# Shared by the bar scripts. Source, don't run. Messages go to stderr: `eww logs`.
# shellcheck shell=bash

EWW="${EWW:-eww}"
POPUPS=(sound power_menu popup_closer)   # every popup window, closed together

log() { echo "${0##*/}: $*" >&2; }

# need <command>...: a listener that cannot work says why once and then idles, so
# eww does not restart it in a loop.
need() {
  local c
  for c; do
    command -v "$c" >/dev/null && continue
    log "$c missing — run ./eww/install.sh to see what to install"
    exec sleep infinity
  done
}

eww_running() { "$EWW" ping >/dev/null 2>&1; }

hypr_socket() { echo "${XDG_RUNTIME_DIR:-/run/user/$UID}/hypr/${HYPRLAND_INSTANCE_SIGNATURE:-}/.socket2.sock"; }

# hypr_events: Hyprland's event stream, one "event>>data" line each.
hypr_events() { socat -u UNIX-CONNECT:"$(hypr_socket)" -; }

# print_on_change: copy stdin to stdout, skipping a line equal to the one before.
print_on_change() {
  local line last="" first=1
  while IFS= read -r line; do
    if (( first )) || [[ "$line" != "$last" ]]; then
      printf '%s\n' "$line"
      last="$line" first=0
    fi
  done
}
