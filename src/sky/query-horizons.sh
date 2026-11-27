#!/usr/bin/env bash
# Ask JPL Horizons where Voyager 1 and Voyager 2 are in the sky and save it for the wallpaper.
#
#   src/sky/query-horizons.sh                 # now, seen from Taipei -> awww/sky/voyager{1,2}.json
#   src/sky/query-horizons.sh 2027-06-01      # at another date (UTC midnight)
#
# Voyager 1 is ~173 AU away and Voyager 2 ~145 AU: from anywhere on Earth each sits at
# the same point of the sky to within 0.05", and drifts only ~0.3 deg a year (Earth's
# own orbit, seen from out there). So this is run once and the result committed; the script stays in
# case the position ever needs refreshing.
#
# Horizons API docs: https://ssd-api.jpl.nasa.gov/doc/horizons.html
# Needs: curl, awk.

set -euo pipefail

ROOT="$(readlink -f "$(dirname "$0")/../..")"

# observer: Taipei (east longitude, latitude in degrees, altitude in km)
SITE="121.5654,25.0330,0.01"
SITE_NAME="Taipei, Taiwan"

day="${1:-$(date -u +%Y-%m-%d)}"
start="$day 00:00"
stop="$day 00:01"

trim() { local s="$1"; s="${s#"${s%%[![:space:]]*}"}"; echo "${s%"${s##*[![:space:]]}"}"; }

# query N: Voyager N (Horizons id -30-N) -> awww/sky/voyagerN.json
query() {
local n="$1" id=$((-30 - $1))
local out="$ROOT/awww/sky/voyager$n.json"
local reply row date ra dec au glon glat
reply="$(curl -sSf -G 'https://ssd.jpl.nasa.gov/api/horizons.api' \
  --data-urlencode "format=text" \
  --data-urlencode "COMMAND='$id'" \
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

mkdir -p "$(dirname "$out")"
cat > "$out" <<EOF
{
  "name": "Voyager $n",
  "horizons_id": $id,
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
echo "wrote $out"
cat "$out"
}

query 1
query 2
