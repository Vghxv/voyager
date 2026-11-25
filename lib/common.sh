# Shared helpers for voyager's install.sh and check.sh scripts. Source, don't run.
# shellcheck shell=bash

VOYAGER_LIB="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
VOYAGER_CONF="$HOME/.config/voyager"          # saved mode + theme hooks
THEME_HOOKS="$VOYAGER_CONF/theme.d"           # one hook per component
THEME_CMD="$HOME/.config/scripts/voyager-theme"
HYPR_VOYAGER="$HOME/.config/hypr/voyager"     # shared by awww and hyprlock
SYSTEMD_USER="$HOME/.config/systemd/user"
PLYMOUTH_THEME_DIR=/usr/share/plymouth/themes/voyager

# Colours only on a terminal, and never with NO_COLOR set.
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  C_RED=$'\033[31m' C_GREEN=$'\033[32m' C_YELLOW=$'\033[33m' C_DIM=$'\033[2m' C_OFF=$'\033[0m'
else
  C_RED="" C_GREEN="" C_YELLOW="" C_DIM="" C_OFF=""
fi

red()    { printf '%s%s%s\n' "$C_RED" "$*" "$C_OFF"; }
green()  { printf '%s%s%s\n' "$C_GREEN" "$*" "$C_OFF"; }
yellow() { printf '%s%s%s\n' "$C_YELLOW" "$*" "$C_OFF"; }

require_root() { [[ $EUID -eq 0 ]] || { red "Run with sudo."; exit 1; }; }
require_user() { [[ $EUID -ne 0 ]] || { red "Run as your normal user, not root."; exit 1; }; }

# parse_option <allowed>... -- "$@": accept nothing or one of <allowed>, put it in
# OPT, and stop with a message on anything else.
parse_option() {
  local allowed=() a hint
  while [[ $1 != -- ]]; do allowed+=("$1"); shift; done
  shift
  if (( ${#allowed[@]} )); then
    hint="use $(printf '%s, ' "${allowed[@]}")"; hint="${hint%, } or nothing"
  else
    hint="this installer takes none"
  fi
  (( $# <= 1 )) || { red "Too many options: $* ($hint)"; exit 1; }
  OPT="${1:-}"
  for a in "" "${allowed[@]}"; do [[ "$OPT" == "$a" ]] && return 0; done
  red "Unknown option: $OPT ($hint)"
  exit 1
}

# read_mode: the saved light/dark mode. Anything but "dark" is light, as in
# voyager-theme (which keeps its own copy: it runs without this file).
read_mode() {
  if [[ "$(cat "$VOYAGER_CONF/theme" 2>/dev/null)" == dark ]]; then echo dark; else echo light; fi
}

# ---------- dependencies ----------
# A component's `deps` file has one line per command it calls:
#   command[|alternative] package [required]
# e.g. "nmcli|iw iw" is satisfied by either command.

# missing_deps <deps file>: print "package" or "package required" per missing line.
missing_deps() {
  local cmds pkg req alt alts found
  while read -r cmds pkg req; do
    [[ -z "$cmds" || "$cmds" == \#* ]] && continue
    IFS='|' read -ra alts <<<"$cmds"
    found=0
    for alt in "${alts[@]}"; do
      if command -v "$alt" >/dev/null; then found=1; break; fi
    done
    (( found )) || echo "$pkg${req:+ $req}"
  done < "$1"
}

# missing_split <deps file>: fill the arrays NEED (required) and WANT (optional).
missing_split() {
  local pkg req
  NEED=() WANT=()
  while read -r pkg req; do
    if [[ "$req" == required ]]; then NEED+=("$pkg"); else WANT+=("$pkg"); fi
  done < <(missing_deps "$1")
}

# check_deps <deps file>: stop if a required package is missing, warn about the rest.
check_deps() {
  missing_split "$1"
  if (( ${#NEED[@]} )); then
    red "Missing: ${NEED[*]}"
    echo "  sudo pacman -S ${NEED[*]}${WANT[*]:+ ${WANT[*]}}"
    exit 1
  fi
  if (( ${#WANT[@]} )); then
    yellow "Missing (parts will not work until installed): ${WANT[*]}"
    echo "  sudo pacman -S ${WANT[*]}"
  fi
}

# ---------- backups ----------
# backup_file <target> <new-content>
# Copies <target> to <target>.bak-YYYYmmdd-HHMMSS, unless it does not exist or is
# already voyager's (same content, or an older voyager version: first line starts
# with "# voyager"). Backing up voyager's own file would make --restore bring
# voyager back instead of your original.
backup_file() {
  local target="$1" new="$2" bak
  [[ -f "$target" ]] || return 0
  if cmp -s "$target" "$new" || [[ "$(head -1 "$target")" == "# voyager"* ]]; then
    echo "  $target is already voyager's — no backup needed"
    return 0
  fi
  bak="$target.bak-$(date +%Y%m%d-%H%M%S)"
  cp -p "$target" "$bak"
  green "  backed up $target -> $bak"
}

# latest_backup <target>: print the newest <target>.bak-* (names sort by date), if any.
latest_backup() {
  local baks
  baks=("$1".bak-*)
  [[ -e "${baks[0]}" ]] && echo "${baks[-1]}"
}

# restore_latest <target>: put the newest <target>.bak-* back.
restore_latest() {
  local target="$1" last
  last="$(latest_backup "$target")" || { red "No backup found ($target.bak-*)."; return 1; }
  cp -p "$last" "$target"
  green "Restored $last -> $target"
}

# ---------- light / dark ----------
# install_theme_hook <name> <hook script>: install the shared voyager-theme
# switcher and this component's hook, then apply the saved mode.
install_theme_hook() {
  install -D -m 755 "$VOYAGER_LIB/voyager-theme" "$THEME_CMD"
  install -D -m 755 "$2" "$THEME_HOOKS/$1"
  "$THEME_CMD" apply || yellow "  ! applying the $("$THEME_CMD") mode failed (see above); the files are installed"
}

# ---------- Plymouth ----------
# initramfs_images: every kernel's main initramfs (linux, linux-lts, ...), not the fallbacks.
initramfs_images() {
  local img
  for img in /boot/initramfs-*.img; do
    if [[ -e "$img" && "$img" != *-fallback.img ]]; then echo "$img"; fi
  done
}

# has_voyager_plymouth <initramfs>: the voyager theme is built into it.
has_voyager_plymouth() { lsinitcpio "$1" 2>/dev/null | grep -q "themes/voyager/voyager.script"; }

# ---------- screen size ----------
# warn_if_small <png> <re-render command>: warn when the largest connected monitor
# is bigger than <png>, which then gets upscaled (soft). Silent without Hyprland or jq.
warn_if_small() {
  local png="$1" cmd="$2" pw ph mw mh
  command -v hyprctl >/dev/null && command -v jq >/dev/null || return 0
  read -r pw ph < <(od -An -tu1 -j16 -N8 "$png" \
    | awk '{ print $1*16777216 + $2*65536 + $3*256 + $4, $5*16777216 + $6*65536 + $7*256 + $8 }') || return 0
  read -r mw mh < <(hyprctl monitors -j 2>/dev/null | jq -r '
    map(if .transform % 2 == 1 then [.height, .width] else [.width, .height] end)
    | max_by(.[0] * .[1]) // empty | "\(.[0]) \(.[1])"') || return 0
  [[ -n "$mw" && -n "$pw" ]] || return 0
  if (( mw > pw || mh > ph )); then
    yellow "  ! $(basename "$png") is ${pw}x${ph} but your largest monitor is ${mw}x${mh} — it will look soft."
    echo "    Re-render for your screens, then re-run this installer:  $cmd"
  fi
}

# ---------- check.sh output (used by ./doctor.sh) ----------
# pass <what>              ✓ line
# fail <what> <fix>        ✗ line plus the command that fixes it
# info <what>              – line (not installed, nothing to check)
# not_installed <fix>      info line, then end the check successfully
pass() { printf '  %s✓%s %s\n' "$C_GREEN" "$C_OFF" "$1"; }
fail() { printf '  %s✗%s %s\n      fix: %s\n' "$C_RED" "$C_OFF" "$1" "$2"; FAILED=1; }
info() { printf '  %s– %s%s\n' "$C_DIM" "$1" "$C_OFF"; }
not_installed() { info "not installed ($1)"; exit 0; }
FAILED=0

# check_deps_report <deps file> <component>: one ✗ naming missing packages, else ✓.
check_deps_report() {
  local what="" all
  missing_split "$1"
  (( ${#NEED[@]} )) && what="${NEED[*]} (required)"
  (( ${#WANT[@]} )) && what+="${what:+, }${WANT[*]} (optional)"
  all="${NEED[*]} ${WANT[*]}"
  if [[ -n "$what" ]]; then fail "$2: missing packages: $what" "sudo pacman -S $(echo $all)"
  else pass "$2: dependencies installed"; fi
}

# check_same <repo file> <installed file> <fix>: installed copy matches the repo.
check_same() {
  if [[ ! -e "$2" ]]; then fail "$2 is missing" "$3"
  elif ! cmp -s "$1" "$2"; then fail "$2 differs from the repo" "$3"
  else pass "${2/#$HOME/\~} is current"; fi
}

# check_files <label> <fix> <repo dir> <installed dir> <file>...: one ✓ when every
# installed copy matches the repo, else one ✗ naming the files that are off.
check_files() {
  local label="$1" fix="$2" src="$3" dest="$4" f bad=()
  shift 4
  for f in "$@"; do cmp -s "$src/$f" "$dest/$f" || bad+=("$f"); done
  if (( ${#bad[@]} )); then fail "$label: missing or changed: ${bad[*]}" "$fix"
  else pass "$label match the repo"; fi
}

# run_checks: the last line of every check.sh — exit 1 if anything failed.
run_checks() { exit "$FAILED"; }
