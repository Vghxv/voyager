#!/bin/bash
# Emit the workspace state as JSON on every change:
#   {"workspaces":[1,2,3],"active":2,"monitors":{"eDP-1":1,"HDMI-A-1":2}}
# "active" is the focused workspace, "monitors" the workspace each monitor shows
# (each bar highlights the one on its own monitor).

SOCKET="$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock"

get_state() {
    workspaces=$(hyprctl workspaces -j | jq -c 'map(select(.id > 0) | .id) | sort')
    active=$(hyprctl activeworkspace -j | jq -c '.id')
    monitors=$(hyprctl monitors -j | jq -c 'map({(.name): .activeWorkspace.id}) | add // {}')
    echo "{\"workspaces\":$workspaces,\"active\":$active,\"monitors\":$monitors}"
}

last_state=$(get_state)
echo "$last_state"

# Listen for Hyprland events
socat -u UNIX-CONNECT:"$SOCKET" - | while read -r line; do
    case "$line" in
        workspace* | createworkspace* | destroyworkspace* | moveworkspace* | \
        focusedmon* | monitoradded* | monitorremoved*)
            current_state=$(get_state)
            if [ "$current_state" != "$last_state" ]; then
                echo "$current_state"
                last_state="$current_state"
            fi
            ;;
    esac
done
