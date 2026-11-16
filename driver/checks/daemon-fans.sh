#!/usr/bin/env bash
# On-device check of the daemon's fan control (#9): the curve holds manual mode, the kernel watchdog
# returns the fans to automatic when avelld is frozen, avelld takes them back, and every mode of
# `avellctl fan set` reaches the driver. Ends with the fans on automatic (EXIT trap).
# Afterwards: `avellctl fan set curve` gives the fans back to the daemon's curve.
set -euo pipefail

H=$(for h in /sys/class/hwmon/hwmon*; do [ "$(cat "$h/name")" = uniwill ] && echo "$h"; done)
[ -n "$H" ] || { echo "no uniwill hwmon device"; exit 1; }
PID=$(systemctl show -p MainPID --value avelld.service)
[ "$PID" != 0 ] || { echo "avelld is not running"; exit 1; }

restore() {
  sudo kill -CONT "$PID" 2>/dev/null || true
  avellctl fan set automatic 2>/dev/null || echo 2 | sudo tee "$H/pwm1_enable" >/dev/null
  echo "restored: avelld running, fans on automatic (mode $(cat "$H/pwm1_enable"))"
}
trap restore EXIT

mode() { cat "$H/pwm1_enable"; }
expect() { # expect <what> <want> <got>
  if [ "$2" = "$3" ]; then echo "ok    $1: $3"; else echo "FAIL  $1: wanted $2, got $3"; exit 1; fi
}

avellctl fan set curve; sleep 3
expect "curve holds manual mode" 1 "$(mode)"
avellctl fan

sudo kill -STOP "$PID"; sleep 8
expect "watchdog with avelld frozen" 2 "$(mode)"
sudo dmesg | grep "no fan duty written" | tail -1

sudo kill -CONT "$PID"; sleep 4
expect "avelld takes the fans back" 1 "$(mode)"

avellctl fan set 50; sleep 3
expect "fixed speed is manual mode" 1 "$(mode)"
avellctl fan

avellctl fan set max; sleep 4
expect "Max fan" 0 "$(mode)"
avellctl fan

avellctl fan set automatic; sleep 3
expect "automatic" 2 "$(mode)"

# Max fan turned on by someone else is left alone under the curve, and the curve returns after
avellctl fan set curve; sleep 3
echo 0 | sudo tee "$H/pwm1_enable" >/dev/null; sleep 5
expect "someone else's Max fan is kept" 0 "$(mode)"
echo 2 | sudo tee "$H/pwm1_enable" >/dev/null; sleep 4
expect "the curve returns after it" 1 "$(mode)"

sudo systemctl restart avelld; sleep 3
PID=$(systemctl show -p MainPID --value avelld.service)
expect "a restart starts on the curve" 1 "$(mode)"
sudo systemctl stop avelld; sleep 1
expect "a clean stop leaves automatic" 2 "$(mode)"
sudo systemctl start avelld; sleep 3
PID=$(systemctl show -p MainPID --value avelld.service)
echo "all checks passed"
