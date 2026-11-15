#!/usr/bin/env bash
# On-device check of the TGP offset (EC 0x0744 via the driver's ctgp_offset attribute).
# Shows the graphics driver's power limits before and after, checks the clamp,
# and always restores the previous value. Wakes the graphics card (nvidia-smi).
set -euo pipefail

A=$(ls /sys/bus/platform/devices/*/ctgp_offset 2>/dev/null | head -1 || true)
[ -n "$A" ] || { echo "No ctgp_offset attribute: TGP offset is not enabled in this driver build."; exit 1; }
OFFSET=${1:-20}
prev=$(cat "$A")
trap 'echo "$prev" | sudo tee "$A" >/dev/null; echo "restored TGP offset to $prev"' EXIT

limits() { nvidia-smi --query-gpu=power.limit,power.default_limit,power.max_limit,enforced.power.limit,power.draw --format=csv,noheader; }

echo "TGP offset now: $prev W"
echo "before:  limit, default, max, enforced, draw = $(limits)"
echo "$OFFSET" | sudo tee "$A" >/dev/null
sleep 5
echo "TGP offset set: $(cat "$A") W"
echo "after:   limit, default, max, enforced, draw = $(limits)"

if echo 21 | sudo tee "$A" >/dev/null 2>&1; then echo "FAIL: 21 W was accepted"; exit 1; else echo "21 W rejected (as expected)"; fi
