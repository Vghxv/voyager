#!/usr/bin/env bash
# Emit the workspace state as JSON on every change:
#   {"workspaces":[1,2,3],"active":2,"monitors":{"eDP-1":1,"HDMI-A-1":2}}
# "active" is the focused workspace, "monitors" the workspace each monitor shows
# (each bar highlights the one on its own monitor).

#
# Needs: hyprctl, jq, socat. Errors go to `eww logs`.

SOCKET="$XDG_RUNTIME_DIR/hypr/${HYPRLAND_INSTANCE_SIGNATURE:-}/.socket2.sock"
for c in hyprctl jq socat; do
  command -v $c >/dev/null || { echo "workspaces.sh: $c missing — run ./eww/install.sh to see what to install" >&2; exec sleep infinity; }
done
[[ -S "$SOCKET" ]] || { echo "workspaces.sh: no Hyprland event socket ($SOCKET) — is eww running inside Hyprland?" >&2; exec sleep infinity; }

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
