#!/usr/bin/env bash
# Remove the control stack: the daemon, its service, D-Bus policy and CLI, then the patched driver; the kernel's own uniwill-laptop stays (it will not auto-load on this board).
set -euo pipefail

NAME=uniwill-avell
VERSION=$(sed -n 's/^PACKAGE_VERSION="\(.*\)"/\1/p' "$(dirname "$0")/../driver/dkms.conf")

# Daemon first: it hands the fans back to automatic when it stops
sudo systemctl disable --now avelld.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/avelld.service /usr/share/dbus-1/system.d/org.avell.Control1.conf \
  /usr/local/bin/avelld /usr/local/bin/avellctl
sudo rm -rf /etc/avelld
sudo systemctl daemon-reload

# Fans back to automatic before the driver goes away (the driver also does this on unload)
for h in /sys/class/hwmon/hwmon*; do
  [ "$(cat "$h/name" 2>/dev/null)" = uniwill ] && [ -w "$h/pwm1_enable" ] && echo 2 | sudo tee "$h/pwm1_enable" >/dev/null || true
done

sudo systemctl disable --now nvidia-powerd.service 2>/dev/null || true
sudo modprobe -r uniwill-laptop 2>/dev/null || true
sudo dkms remove "$NAME/$VERSION" --all 2>/dev/null || true
sudo rm -rf "/usr/src/$NAME-$VERSION"
sudo rm -f /etc/modules-load.d/uniwill-avell.conf
echo "Removed $NAME $VERSION"
