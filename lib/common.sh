# Shared helpers for the voyager install scripts. Source, don't execute.

red()    { printf '\033[31m%s\033[0m\n' "$*"; }
green()  { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }

require_root() { [[ $EUID -eq 0 ]] || { red "Run with sudo."; exit 1; }; }
require_user() { [[ $EUID -ne 0 ]] || { red "Run as your normal user, not root."; exit 1; }; }

# backup_file <target> <new-content>
# Copies <target> to <target>.bak-YYYYmmdd-HHMMSS, unless it does not exist or
# already equals <new-content> (re-installing must never back up voyager's own
# file, or --restore would bring voyager back instead of your original).
backup_file() {
  local target="$1" new="$2" bak
  [[ -f "$target" ]] || return 0
  if cmp -s "$target" "$new"; then
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
