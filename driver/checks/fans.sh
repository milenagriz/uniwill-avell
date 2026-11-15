#!/usr/bin/env bash
# On-device check of the fan mode contract (pwmN_enable: 0 = Max fan, 1 = manual, 2 = automatic).
# Writes only EC 0x0751 bit 6 (Max fan), which is verified on this board. Always ends on automatic.
set -euo pipefail

H=""
for h in /sys/class/hwmon/hwmon*; do
  [ "$(cat "$h/name" 2>/dev/null)" = uniwill ] && H=$h
done
[ -n "$H" ] || { echo "No 'uniwill' hwmon device. Is the driver loaded?"; exit 1; }
[ -e "$H/pwm1_enable" ] || { echo "No pwm1_enable: this is not the patched driver."; exit 1; }

trap 'echo 2 | sudo tee "$H/pwm1_enable" >/dev/null' EXIT   # always end on automatic

fail=0
set_mode() { echo "$1" | sudo tee "$H/pwm1_enable" >/dev/null; }
show() { printf "%-14s fan1=%5s fan2=%5s pwm1=%3s pwm1_enable=%s pwm2_enable=%s\n" "$1" \
  "$(cat "$H/fan1_input")" "$(cat "$H/fan2_input")" "$(cat "$H/pwm1")" "$(cat "$H/pwm1_enable")" "$(cat "$H/pwm2_enable")"; }
expect() {  # expect <label> <mode> [min_pwm]
  local m p; m=$(cat "$H/pwm1_enable"); p=$(cat "$H/pwm1")
  show "$1"
  if [ "$m" != "$2" ] || [ "$(cat "$H/pwm2_enable")" != "$2" ]; then echo "  FAIL: expected mode $2"; fail=1; fi
  if [ -n "${3:-}" ] && [ "$p" -lt "$3" ]; then echo "  FAIL: expected pwm1 >= $3"; fail=1; fi
}

set_mode 2; sleep 1; expect auto 2
set_mode 1; sleep 3; expect "manual" 1 250            # manual starts at full speed
echo 255 | sudo tee "$H/pwm1" >/dev/null; sleep 1; expect "manual pwm=255" 1 250
set_mode 2; sleep 3; expect "auto" 2
set_mode 0; sleep 3; expect "max" 0 250
set_mode 1; sleep 1; expect "max->manual" 1 250
set_mode 2; sleep 3; expect "manual->auto" 2

# The Cooling widget's sequence, repeated: enable=1, pwm=255 ... pwm=76 (rejected), enable=2
for i in 1 2 3; do
  set_mode 1; echo 255 | sudo tee "$H/pwm1" >/dev/null; sleep 2; expect "widget max #$i" 1 250
  echo 76 | sudo tee "$H/pwm1" >/dev/null 2>&1 || true
  set_mode 2; sleep 2; expect "widget auto #$i" 2
done

[ "$fail" = 0 ] && echo PASS || { echo FAIL; exit 1; }
