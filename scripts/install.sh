#!/usr/bin/env bash
# Install the Avell A52 MOB control stack. Today: the patched uniwill-laptop driver via DKMS.
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

# Swap the running module for the new one
sudo modprobe -r uniwill-laptop 2>/dev/null || true
sudo modprobe uniwill-laptop

echo "Loaded uniwill-laptop $(cat /sys/module/uniwill_laptop/version 2>/dev/null || echo '?')"
