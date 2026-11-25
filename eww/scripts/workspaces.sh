#!/usr/bin/env bash
# Emit the workspace state as JSON on every change:
#   {"workspaces":[1,2,3],"active":2,"monitors":{"eDP-1":1,"HDMI-A-1":2}}
# "active" is the focused workspace, "monitors" the workspace each monitor shows
# (each bar highlights the one on its own monitor).
#
# Needs: hyprctl, jq, socat. Errors go to `eww logs`.

source "$(dirname "$0")/lib.sh"

need hyprctl jq socat
[[ -S "$(hypr_socket)" ]] || { log "no Hyprland event socket ($(hypr_socket)) — is eww running inside Hyprland?"; exec sleep infinity; }

# One JSON line; nothing if hyprctl fails (the bar keeps the last state).
get_state() {
  local w a m
  w=$(hyprctl workspaces -j) && a=$(hyprctl activeworkspace -j) && m=$(hyprctl monitors -j) || return
  jq -nc --argjson w "$w" --argjson a "$a" --argjson m "$m" '{
    workspaces: ($w | map(select(.id > 0) | .id) | sort),
    active: $a.id,
    monitors: ($m | map({(.name): .activeWorkspace.id}) | add // {})
  }'
}

# Hyprland sends most events twice (v1 and v2): react to one of each.
{
  echo start
  hypr_events
} | while read -r line; do
  case "$line" in
    start | workspacev2'>>'* | createworkspacev2'>>'* | destroyworkspacev2'>>'* | \
    moveworkspacev2'>>'* | renameworkspace'>>'* | focusedmon'>>'* | \
    monitoradded'>>'* | monitorremoved'>>'*)
      get_state ;;
  esac
done | print_on_change
