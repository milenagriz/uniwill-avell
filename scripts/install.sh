#!/usr/bin/env bash
# Install the Avell A52 MOB control stack: the patched uniwill-laptop driver (DKMS),
# the avelld daemon with its systemd service and D-Bus policy, and the avellctl CLI.
set -euo pipefail

NAME=uniwill-avell
REPO=$(cd "$(dirname "$0")/.." && pwd)
DRIVER=$REPO/driver
VERSION=$(sed -n 's/^PACKAGE_VERSION="\(.*\)"/\1/p' "$DRIVER/dkms.conf")
DEST=/usr/src/$NAME-$VERSION

command -v dkms >/dev/null || { echo "dkms is not installed (sudo pacman -S dkms)"; exit 1; }

board=$(cat /sys/class/dmi/id/board_name 2>/dev/null || true)
if [ "$board" != "Avell A52 MOB" ]; then
  echo "Warning: board is '$board', not 'Avell A52 MOB'. The driver will not bind here."
  read -rp "Continue anyway? [y/N] " a; [ "${a,,}" = y ] || exit 1
fi

# Replace an earlier install of the same version
if dkms status "$NAME/$VERSION" 2>/dev/null | grep -q .; then
  sudo dkms remove "$NAME/$VERSION" --all
fi

sudo rm -rf "$DEST"
sudo mkdir -p "$DEST"
sudo cp "$DRIVER"/src/Makefile "$DRIVER"/src/*.[ch] "$DRIVER"/dkms.conf "$DEST"/

sudo dkms install "$NAME/$VERSION"
echo uniwill-laptop | sudo tee /etc/modules-load.d/uniwill-avell.conf >/dev/null

# Dynamic Boost: NVIDIA's power daemon moves spare CPU budget to the graphics card (on AC)
if systemctl list-unit-files nvidia-powerd.service >/dev/null 2>&1; then
  sudo systemctl enable --now nvidia-powerd.service
else
  echo "Warning: nvidia-powerd.service not found; Dynamic Boost stays off (install nvidia-utils)."
fi

# Swap the running module for the new one
sudo modprobe -r uniwill-laptop 2>/dev/null || true
sudo modprobe uniwill-laptop

echo "Loaded uniwill-laptop $(cat /sys/module/uniwill_laptop/version 2>/dev/null || echo '?')"

# Daemon and CLI: built as the calling user, installed as root
command -v cargo >/dev/null || { echo "cargo is not installed (see mise.toml)"; exit 1; }
(cd "$REPO/daemon" && cargo build --release --locked)
sudo install -m 755 "$REPO/daemon/target/release/avelld" "$REPO/daemon/target/release/avellctl" /usr/local/bin/
sudo install -m 644 "$REPO/daemon/dist/org.avell.Control1.conf" /usr/share/dbus-1/system.d/
sudo install -m 644 "$REPO/daemon/dist/avelld.service" /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable avelld.service
sudo systemctl restart avelld.service
echo "avelld $(systemctl is-active avelld.service), avellctl in /usr/local/bin"
