#!/usr/bin/env bash
# Print time until empty or full for the battery tooltip ("Depleted in - 3h:12m").
# Reads energy_* (or charge_*) from sysfs; prints "No battery" without one.
name=$("$(dirname "$0")/battery_name.sh")
[[ -n "$name" ]] || { echo "No battery"; exit 0; }
BAT=/sys/class/power_supply/$name

read_file() {
  local f="$1"
  [[ -f "$f" ]] && cat "$f" || echo 0
}

status=$(read_file "$BAT/status")

now=$(read_file "$BAT/energy_now")
full=$(read_file "$BAT/energy_full")
rate=$(read_file "$BAT/power_now")

# fallback if energy_* doesn't exist
if [[ "$now" == "0" || "$rate" == "0" ]]; then
  now=$(read_file "$BAT/charge_now")
  full=$(read_file "$BAT/charge_full")
  rate=$(read_file "$BAT/current_now")
fi

if [[ "$rate" -le 0 ]]; then
  echo "Eternity"
  exit 0
fi

if [[ "$status" == "Discharging" ]]; then
  awk -v n="$now" -v r="$rate" '
    BEGIN {
        t = (n / r) * 3600;
        h = int(t / 3600);
        m = int((t % 3600) / 60);
        printf "Depleted in - %dh:%02dm\n", h, m;
    }'
elif [[ "$status" == "Charging" ]]; then
  awk -v n="$now" -v f="$full" -v r="$rate" '
    BEGIN {
        t = ((f - n) / r) * 3600;
        h = int(t / 3600);
        m = int((t % 3600) / 60);
        printf "Fully Charged in - %dh:%02dm\n", h, m;
    }'
else
  echo "$status"
fi
