#!/usr/bin/env bash
# On-device check of battery care: the charging profile (EC 0x07A6 bits 4-5, through BAT0/charge_types)
# and the real cycle count. Always restores the original charging profile.
# The charging behaviour itself (stopping at the profile's level) needs a battery below that level;
# see docs/INVESTIGATION.md for the discharge procedure.
set -euo pipefail
B=/sys/class/power_supply/BAT0
D=/sys/bus/platform/devices/INOU0000:00
[ -e $B/charge_types ] || { echo "No charge_types on BAT0: battery care not enabled in this driver build."; exit 1; }
orig=$(sed -E 's/.*\[([^]]+)\].*/\1/' $B/charge_types)
trap 'echo "$orig" | sudo tee $B/charge_types >/dev/null; echo "restored: $(cat $B/charge_types)"' EXIT
ec() { sudo grep "^$1:" /sys/kernel/debug/regmap/INOU0000:00/registers | cut -d' ' -f2; }
fail=0
echo "cycle count: driver $(cat $D/battery_cycle_count), ACPI $(cat $B/cycle_count); wear: $(( $(cat $B/charge_full) * 100 / $(cat $B/charge_full_design) ))% of design"
echo "before: $(cat $B/charge_types)  ec7a6=$(ec 7a6)  status=$(cat $B/status) capacity=$(cat $B/capacity)%"
for t in Long_Life Trickle Standard; do
  echo "$t" | sudo tee $B/charge_types >/dev/null; sleep 3
  got=$(sed -E 's/.*\[([^]]+)\].*/\1/' $B/charge_types)
  printf "%-10s -> %-28s ec7a6=%s status=%s\n" "$t" "$(cat $B/charge_types)" "$(ec 7a6)" "$(cat $B/status)"
  [ "$got" = "$t" ] || { echo "  FAIL"; fail=1; }
done
[ "$fail" = 0 ] && echo PASS || { echo FAIL; exit 1; }
