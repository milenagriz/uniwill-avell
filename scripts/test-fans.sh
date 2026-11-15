#!/usr/bin/env bash
# Live check of the fan boost interface: full speed for ~10 s, then back to automatic.
set -euo pipefail

H=""
for h in /sys/class/hwmon/hwmon*; do
  [ "$(cat "$h/name" 2>/dev/null)" = uniwill ] && H=$h
done
[ -n "$H" ] || { echo "No 'uniwill' hwmon device. Is the driver loaded?"; exit 1; }
[ -e "$H/pwm1_enable" ] || { echo "No pwm1_enable: this is not the patched driver."; exit 1; }

trap 'echo 2 | sudo tee "$H/pwm1_enable" >/dev/null' EXIT   # always end on automatic

show() { printf "%-8s fan1=%5s fan2=%5s pwm1=%3s pwm1_enable=%s\n" "$1" \
  "$(cat "$H/fan1_input")" "$(cat "$H/fan2_input")" "$(cat "$H/pwm1")" "$(cat "$H/pwm1_enable")"; }

show auto
echo 0 | sudo tee "$H/pwm1_enable" >/dev/null
for i in 1 2 3; do sleep 3; show "max+${i}x3s"; done
echo 2 | sudo tee "$H/pwm1_enable" >/dev/null
sleep 3; show auto
