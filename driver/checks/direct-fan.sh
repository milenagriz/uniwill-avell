#!/usr/bin/env bash
# On-device check of direct fan speed (local-only driver series): manual duty through the universal fan
# tables, Max fan while entering manual and above the hard temperature, and the kernel watchdog.
# Writes EC 0x0751 bit 6, 0x07C5, 0x07C6 and the fan tables 0x0F00-0x0F5F. Always ends on automatic.
# --sweep: also step both fans through low duties to find the lowest spinning duty (takes about 6 minutes).
set -euo pipefail

H=""
for h in /sys/class/hwmon/hwmon*; do [ "$(cat "$h/name" 2>/dev/null)" = uniwill ] && H=$h; done
[ -n "$H" ] || { echo "No 'uniwill' hwmon device."; exit 1; }
P=/sys/module/uniwill_laptop/parameters
[ -e $P/fan_watchdog_sec ] || { echo "No fan_watchdog_sec parameter: this driver has no direct fan speed."; exit 1; }
WD=$(cat $P/fan_watchdog_sec); HOT=$(cat $P/fan_hard_temp_cpu)

trap 'echo "$HOT" | sudo tee $P/fan_hard_temp_cpu >/dev/null; echo 2 | sudo tee "$H/pwm1_enable" >/dev/null; echo "end: pwm1_enable=$(cat $H/pwm1_enable)"' EXIT
put() { echo "$1" | sudo tee "$H/pwm1" >/dev/null; echo "$2" | sudo tee "$H/pwm2" >/dev/null; }
show() { printf "%-26s mode=%s fan1=%5s fan2=%5s pwm1=%3s pwm2=%3s cpu=%s\n" "$1" "$(cat $H/pwm1_enable)" \
  "$(cat $H/fan1_input)" "$(cat $H/fan2_input)" "$(cat $H/pwm1)" "$(cat $H/pwm2)" "$(( $(cat $H/temp1_input) / 1000 ))"; }
hold() {  # hold <pwm1> <pwm2> <seconds> [label]: keep writing every 2 s, show every 6 s
  local i; for i in $(seq 2 2 "$3"); do put "$1" "$2"; sleep 2; if [ $(( i % 6 )) = 0 ]; then show "${4:-$1/$2} +${i}s"; fi; done
}
fail=0
expect() { [ "$(cat $H/pwm1_enable)" = "$1" ] || { echo "  FAIL: mode $(cat $H/pwm1_enable), expected $1"; fail=1; }; }
fast() { [ "$(cat $H/fan1_input)" -gt 4500 ] || { echo "  FAIL: fan1 not at full speed"; fail=1; }; }

echo 2 | sudo tee "$H/pwm1_enable" >/dev/null; sleep 2; show "auto"
echo 1 | sudo tee "$H/pwm1_enable" >/dev/null; sleep 3; show "manual +3s (Max fan start)"
expect 1; fast
hold 0 0 24 "duty 0 (slow ramp down)"; expect 1
hold 255 0 12 "fan1 255 / fan2 0";      expect 1

echo 30 | sudo tee $P/fan_hard_temp_cpu >/dev/null
hold 0 0 6 "hard temp 30 C (forced)"; expect 1; fast
echo "$HOT" | sudo tee $P/fan_hard_temp_cpu >/dev/null
hold 0 0 12 "hard temp back to $HOT C"; expect 1

if [ "${1:-}" = --sweep ]; then
  hold 0 0 90 "stop"
  for d in 8 16 24 32 40 48 64; do hold "$d" "$d" 40 "sweep $d"; done
fi

put 100 100; sleep $(( WD + 3 )); show "silence $(( WD + 3 ))s"
expect 2; [ "$fail" = 0 ] && echo "watchdog returned to automatic"

[ "$fail" = 0 ] && echo PASS || { echo FAIL; exit 1; }
