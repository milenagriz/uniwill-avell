#!/usr/bin/env bash
# All-core CPU load (openssl) for $1 seconds, sampled every 2 s: package power (RAPL, so run with sudo), mean clock,
# package temperature, fans, pwm1 and package throttle events since the start. Finds the hwmon devices by name.
# Usage: sudo bash cpuload.sh 150
set -u
H=$(grep -l '^uniwill$' /sys/class/hwmon/hwmon*/name | xargs dirname)
C=$(grep -l '^coretemp$' /sys/class/hwmon/hwmon*/name | xargs dirname)
E=/sys/class/powercap/intel-rapl:0/energy_uj
T=/sys/devices/system/cpu/cpu0/thermal_throttle/package_throttle_count
openssl speed -evp sha256 -multi $(nproc) -seconds "$1" -bytes 16384 >/dev/null 2>&1 &
P=$!
t0=$(date +%s.%N); e0=$(cat $E); tp=$t0; th0=$(cat $T)
echo "t pkgW MHz pkgC fan1 fan2 pwm1 throttles"
while kill -0 $P 2>/dev/null; do
  sleep 2
  e1=$(cat $E); t=$(date +%s.%N)
  mhz=$(awk '{s+=$1;n++} END{printf "%d", s/n/1000}' /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq)
  awk -v t=$t -v t0=$t0 -v tp=$tp -v e1=$e1 -v e0=$e0 -v mhz=$mhz -v c=$(cat $C/temp1_input) -v f1=$(cat $H/fan1_input) -v f2=$(cat $H/fan2_input) -v p1=$(cat $H/pwm1) -v th=$(( $(cat $T) - th0 )) \
    'BEGIN{printf "%3.0f %5.1f %5d %3d %5d %5d %4d %6d\n", t-t0, (e1-e0)/1e6/(t-tp), mhz, c/1000, f1, f2, p1, th}'
  e0=$e1; tp=$t
done
