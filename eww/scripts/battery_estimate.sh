#!/usr/bin/env bash
# Print time until empty or full for the battery tooltip ("Depleted in - 3h:12m"),
# or the battery's status when it is neither charging nor discharging ("Full",
# "Not charging"). Reads energy_* (or charge_*) from sysfs; "No battery" without one.

name=$("$(dirname "$0")/battery_name.sh")
[[ -n "$name" ]] || { echo "No battery"; exit 0; }
BAT=/sys/class/power_supply/$name

# num <file>: its value as a positive whole number (some drivers report a negative
# current while discharging), 0 if missing or not a number.
num() {
  local v
  v=$(cat "$1" 2>/dev/null)
  v="${v#-}"
  [[ "$v" =~ ^[0-9]+$ ]] && echo "$v" || echo 0
}

status=$(cat "$BAT/status" 2>/dev/null || echo Unknown)
if [[ -e "$BAT/energy_now" ]]; then   # µWh and µW
  now=$(num "$BAT/energy_now") full=$(num "$BAT/energy_full") rate=$(num "$BAT/power_now")
else                                  # µAh and µA
  now=$(num "$BAT/charge_now") full=$(num "$BAT/charge_full") rate=$(num "$BAT/current_now")
fi

case "$status" in
  Discharging) left=$now label="Depleted in" ;;
  Charging)    left=$(( full > now ? full - now : 0 )) label="Fully Charged in" ;;
  *)           echo "$status"; exit 0 ;;
esac
(( rate > 0 )) || { echo "$status"; exit 0; }   # no rate reported yet

t=$(( left * 3600 / rate ))
printf '%s - %dh:%02dm\n' "$label" $(( t / 3600 )) $(( t % 3600 / 60 ))
