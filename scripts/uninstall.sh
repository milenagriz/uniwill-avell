#!/usr/bin/env bash
# Remove the control stack. Today: the patched driver; the kernel's own uniwill-laptop stays (it will not auto-load on this board).
set -euo pipefail

NAME=uniwill-avell
VERSION=$(sed -n 's/^PACKAGE_VERSION="\(.*\)"/\1/p' "$(dirname "$0")/../driver/dkms.conf")

# Fans back to automatic before the driver goes away (the driver also does this on unload)
for h in /sys/class/hwmon/hwmon*; do
  [ "$(cat "$h/name" 2>/dev/null)" = uniwill ] && [ -w "$h/pwm1_enable" ] && echo 2 | sudo tee "$h/pwm1_enable" >/dev/null || true
done

sudo modprobe -r uniwill-laptop 2>/dev/null || true
sudo dkms remove "$NAME/$VERSION" --all 2>/dev/null || true
sudo rm -rf "/usr/src/$NAME-$VERSION"
sudo rm -f /etc/modules-load.d/uniwill-avell.conf
echo "Removed $NAME $VERSION"
