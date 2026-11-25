#!/usr/bin/env bash
# Ask JPL Horizons where Voyager 1 is in the sky and save it for the wallpaper.
#
#   src/query-horizons.sh                 # now, seen from Taipei -> awww/sky/voyager1.json
#   src/query-horizons.sh 2027-06-01      # at another date (UTC midnight)
#
# Voyager 1 is ~173 AU away: from anywhere on Earth it sits at the same point of the
# sky to within 0.05", and it drifts only ~0.3 deg a year (Earth's own orbit, seen
# from out there). So this is run once and the result committed; the script stays in
# case the position ever needs refreshing.
#
# Horizons API docs: https://ssd-api.jpl.nasa.gov/doc/horizons.html
# Needs: curl, awk.

set -euo pipefail

ROOT="$(dirname "$(dirname "$(readlink -f "$0")")")"
OUT="$ROOT/awww/sky/voyager1.json"

# observer: Taipei (east longitude, latitude in degrees, altitude in km)
SITE="121.5654,25.0330,0.01"
SITE_NAME="Taipei, Taiwan"

day="${1:-$(date -u +%Y-%m-%d)}"
start="$day 00:00"
stop="$day 00:01"

reply="$(curl -sSf -G 'https://ssd.jpl.nasa.gov/api/horizons.api' \
  --data-urlencode "format=text" \
  --data-urlencode "COMMAND='-31'" \
  --data-urlencode "OBJ_DATA='NO'" \
  --data-urlencode "MAKE_EPHEM='YES'" \
  --data-urlencode "EPHEM_TYPE='OBSERVER'" \
  --data-urlencode "CENTER='coord@399'" \
  --data-urlencode "COORD_TYPE='GEODETIC'" \
  --data-urlencode "SITE_COORD='$SITE'" \
  --data-urlencode "START_TIME='$start'" \
  --data-urlencode "STOP_TIME='$stop'" \
  --data-urlencode "STEP_SIZE='1'" \
  --data-urlencode "QUANTITIES='1,20,33'" \
  --data-urlencode "ANG_FORMAT='DEG'" \
  --data-urlencode "CSV_FORMAT='YES'")"

# The table sits between $$SOE and $$EOE. With QUANTITIES 1,20,33 and CSV the row is:
#   date, solar flag, lunar flag, RA (ICRF), Dec, delta (au), deldot (km/s), glx lon, glx lat,
row="$(awk '/^\$\$SOE/{on=1; next} /^\$\$EOE/{on=0} on{print; exit}' <<<"$reply")"
[[ -n "$row" ]] || { echo "No ephemeris row in the Horizons reply:" >&2; echo "$reply" >&2; exit 1; }

IFS=',' read -r date _ _ ra dec au _ glon glat _ <<<"$row"
trim() { local s="$1"; s="${s#"${s%%[![:space:]]*}"}"; echo "${s%"${s##*[![:space:]]}"}"; }

mkdir -p "$(dirname "$OUT")"
cat > "$OUT" <<EOF
{
  "name": "Voyager 1",
  "horizons_id": -31,
  "date_utc": "$(trim "$date")",
  "observer": "$SITE_NAME ($SITE, E-lon deg, lat deg, alt km)",
  "ra": $(trim "$ra"),
  "dec": $(trim "$dec"),
  "frame": "ICRF astrometric (J2000)",
  "distance_au": $(trim "$au"),
  "galactic_l": $(trim "$glon"),
  "galactic_b": $(trim "$glat"),
  "source": "JPL Horizons API, https://ssd.jpl.nasa.gov/horizons/"
}
EOF
echo "wrote $OUT"
cat "$OUT"
