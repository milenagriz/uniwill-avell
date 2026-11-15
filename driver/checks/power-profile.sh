#!/usr/bin/env bash
# On-device check of the Power profile (EC 0x0751 bits 4/5/7 through the platform-profile interface).
# Cycles quiet, balanced and performance, checks that Max fan and the profile do not disturb each other,
# and always restores the original profile and automatic fans.
# With --measure, runs a 20 s all-core load in each profile and prints CPU package power, temperature and fans.
set -euo pipefail

PP=/sys/firmware/acpi/platform_profile
REG=/sys/kernel/debug/regmap/INOU0000:00/registers
H=""
for h in /sys/class/hwmon/hwmon*; do [ "$(cat "$h/name" 2>/dev/null)" = uniwill ] && H=$h; done
[ -n "$H" ] || { echo "No 'uniwill' hwmon device."; exit 1; }
grep -qw quiet "$PP"_choices 2>/dev/null || { echo "No quiet/balanced/performance in $PP""_choices: Power profile not enabled."; exit 1; }

orig=$(cat "$PP")
trap 'echo 2 | sudo tee "$H/pwm1_enable" >/dev/null; echo "$orig" | sudo tee "$PP" >/dev/null; echo "restored: profile=$(cat $PP) pwm1_enable=$(cat $H/pwm1_enable)"' EXIT

fail=0
ec751() { sudo grep '^751:' "$REG" | cut -d' ' -f2; }
check() {  # check <profile> <expected 0x751 profile bits>
  echo "$1" | sudo tee "$PP" >/dev/null; sleep 1
  local got bits; got=$(cat "$PP"); bits=$(printf '%02x' $(( 0x$(ec751) & 0xb0 )))
  printf "%-12s read=%-12s ec751=%s bits=%s\n" "$1" "$got" "$(ec751)" "$bits"
  [ "$got" = "$1" ] && [ "$bits" = "$2" ] || { echo "  FAIL"; fail=1; }
}

echo "original: $orig (ec751=$(ec751))"
check quiet a0
check balanced 00
check performance 10

# Max fan must survive a profile change, and the profile must survive Max fan on/off
echo 0 | sudo tee "$H/pwm1_enable" >/dev/null; sleep 1
check quiet a0
[ "$(cat "$H/pwm1_enable")" = 0 ] && echo "Max fan kept across profile change" || { echo "  FAIL: Max fan lost"; fail=1; }
echo 2 | sudo tee "$H/pwm1_enable" >/dev/null; sleep 1
[ "$(cat "$PP")" = quiet ] && echo "profile kept across Max fan off" || { echo "  FAIL: profile lost"; fail=1; }

if [ "${1:-}" = --measure ]; then
  E=/sys/class/powercap/intel-rapl:0/energy_uj
  for p in quiet balanced performance; do
    echo "$p" | sudo tee "$PP" >/dev/null; sleep 20   # let the fans settle
    e0=$(sudo cat $E); t0=$(date +%s.%N)
    openssl speed -evp sha256 -multi "$(nproc)" -seconds 20 -bytes 16384 >/dev/null 2>&1
    e1=$(sudo cat $E); t1=$(date +%s.%N)
    w=$(awk -v a="$e0" -v b="$e1" -v s="$t0" -v t="$t1" 'BEGIN { print (b - a) / 1e6 / (t - s) }')
    printf "%-12s pkg=%5.1f W  cpu=%s C  fan1=%s fan2=%s\n" "$p" "$w" "$(( $(cat $H/temp1_input) / 1000 ))" "$(cat $H/fan1_input)" "$(cat $H/fan2_input)"
  done
fi

[ "$fail" = 0 ] && echo PASS || { echo FAIL; exit 1; }
