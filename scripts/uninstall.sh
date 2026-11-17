#!/usr/bin/env bash
# Remove the control stack: the Avell Control app and the bar plugin, the daemon, its service, D-Bus policy and CLI, then the patched driver; the kernel's own uniwill-laptop stays (it will not auto-load on this board).
set -euo pipefail

# The interfaces first: they are clients of the daemon
for pkg in avell-control avell-control-debug; do
  if pacman -Q "$pkg" >/dev/null 2>&1; then sudo pacman -R --noconfirm "$pkg"; fi
done
if command -v omarchy >/dev/null; then
  omarchy plugin disable avell.control 2>/dev/null || true
fi
link=$HOME/.config/omarchy/plugins/avell.control
if [ -L "$link" ]; then rm -f "$link"; fi

NAME=uniwill-avell
VERSION=$(sed -n 's/^PACKAGE_VERSION="\(.*\)"/\1/p' "$(dirname "$0")/../driver/dkms.conf")

# Daemon first: it hands the fans back to automatic when it stops
sudo systemctl disable --now avelld.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/avelld.service /usr/share/dbus-1/system.d/org.avell.Control1.conf \
  /usr/local/bin/avelld /usr/local/bin/avellctl
sudo rm -rf /etc/avelld
sudo systemctl daemon-reload

# thermald gets the CPU power limit back
sudo rm -f /etc/pacman.d/hooks/avelld-thermald.hook
cdev_order=/etc/thermald/thermal-cpu-cdev-order.xml
if [ -f "$cdev_order" ] && ! grep -q rapl_controller "$cdev_order"; then
  sudo sed -i '0,/<CoolingDevice>/s||<CoolingDevice>rapl_controller</CoolingDevice>\n\t&|' "$cdev_order"
  sudo systemctl try-restart thermald.service
fi

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
