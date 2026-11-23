#!/usr/bin/env bash
# Print one line per missing Plymouth prerequisite; print nothing when all is well.
# Read-only, no root needed. Used by install.sh and check.sh.
#
# Docs: plymouth/plymouth.md#prerequisites

hooks="$(grep -E '^HOOKS=' /etc/mkinitcpio.conf 2>/dev/null)"
for h in systemd plymouth sd-encrypt; do
  grep -qw "$h" <<<"$hooks" || echo "HOOKS in /etc/mkinitcpio.conf is missing '$h'"
done
if grep -qw plymouth <<<"$hooks" && grep -qw sd-encrypt <<<"$hooks"; then
  order="$(grep -oE 'systemd|plymouth|sd-encrypt' <<<"$hooks" | tr '\n' ' ')"
  [[ "$order" == "systemd plymouth sd-encrypt " ]] ||
    echo "HOOKS order should be systemd … plymouth … sd-encrypt (got: $order)"
fi
grep -qw splash /proc/cmdline || echo "'splash' is not on the running kernel cmdline"
