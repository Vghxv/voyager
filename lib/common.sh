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
