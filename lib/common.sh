# Shared helpers for voyager's install.sh and check.sh scripts. Source, don't run.

VOYAGER_LIB="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
VOYAGER_CONF="$HOME/.config/voyager"
THEME_CMD="$HOME/.config/scripts/voyager-theme"

red()    { printf '\033[31m%s\033[0m\n' "$*"; }
green()  { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }

require_root() { [[ $EUID -eq 0 ]] || { red "Run with sudo."; exit 1; }; }
require_user() { [[ $EUID -ne 0 ]] || { red "Run as your normal user, not root."; exit 1; }; }

# ---------- dependencies ----------
# A component's `deps` file has one line per command it calls:
#   command[|alternative] package [required]
# e.g. "nmcli|iw iw" is satisfied by either command.

# missing_deps <deps file>: print "package" or "package required" per missing line.
missing_deps() {
  local cmds pkg req alt found
  while read -r cmds pkg req; do
    [[ -z "$cmds" || "$cmds" == \#* ]] && continue
    found=0
    for alt in ${cmds//|/ }; do command -v "$alt" >/dev/null && { found=1; break; }; done
    (( found )) || echo "$pkg${req:+ $req}"
  done < "$1"
}

# check_deps <deps file>: stop if a required package is missing, warn about the rest.
check_deps() {
  local pkg req need=() want=()
  while read -r pkg req; do
    [[ "$req" == required ]] && need+=("$pkg") || want+=("$pkg")
  done < <(missing_deps "$1")
  if (( ${#need[@]} )); then
    red "Missing: ${need[*]}"
    echo "  sudo pacman -S ${need[*]} ${want[*]}"
    exit 1
  fi
  if (( ${#want[@]} )); then
    yellow "Missing (parts will not work until installed): ${want[*]}"
    echo "  sudo pacman -S ${want[*]}"
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

# restore_latest <target>: put the newest <target>.bak-* back.
restore_latest() {
  local target="$1" last
  last="$(ls -1t "$target".bak-* 2>/dev/null | head -1 || true)"
  [[ -n "$last" ]] || { red "No backup found ($target.bak-*)."; return 1; }
  cp -p "$last" "$target"
  green "Restored $last -> $target"
}

# ---------- light / dark ----------
# install_theme_hook <name> <hook script>: install the shared voyager-theme
# switcher and this component's hook, then apply the saved mode.
install_theme_hook() {
  install -D -m 755 "$VOYAGER_LIB/voyager-theme" "$THEME_CMD"
  rm -f "$HOME/.local/bin/voyager-theme"   # from an earlier version
  install -D -m 755 "$2" "$VOYAGER_CONF/theme.d/$1"
  "$THEME_CMD" apply || yellow "  ! applying the $("$THEME_CMD") mode failed (see above); the files are installed"
}

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
pass() { printf '  \033[32m✓\033[0m %s\n' "$1"; }
fail() { printf '  \033[31m✗\033[0m %s\n      fix: %s\n' "$1" "$2"; FAILED=1; }
info() { printf '  \033[2m– %s\033[0m\n' "$1"; }
FAILED=0

# check_deps_report <deps file> <component>: pass/fail per missing package.
check_deps_report() {
  local missing
  missing="$(missing_deps "$1" | awk '{ print $1 }' | tr '\n' ' ')"
  if [[ -n "$missing" ]]; then fail "$2: missing packages: $missing" "sudo pacman -S $missing"
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
