#!/usr/bin/env bash
# Read-only backup of the whole BIOS flash, from inside the laptop (ticket #17).
# Reads the flash twice with flashrom's internal programmer, compares the reads, and writes the image,
# its SHA-256 and the flashrom log to evidence/bios/. It never writes to the flash: the only flashrom
# invocation is a read (-r) to a new file, and no other flashrom operation exists in this script.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
OUT=$REPO/evidence/bios
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"

read_flash() {  # read_flash <output file> <log file>
  [ ! -e "$1" ] || { echo "refusing to overwrite $1"; exit 1; }
  sudo flashrom --programmer internal --read "$1" 2>&1 | tee "$2"
  sudo chown "$(id -u):$(id -g)" "$1"
}

read_flash "$TMP/read1.bin" "$TMP/read1.log"
read_flash "$TMP/read2.bin" "$TMP/read2.log"

if ! cmp -s "$TMP/read1.bin" "$TMP/read2.bin"; then
  echo "The two reads differ. Nothing was saved; run the backup again."
  exit 1
fi

name=bios-$(cat /sys/class/dmi/id/bios_version)-$(date +%Y%m%d).bin
cp "$TMP/read1.bin" "$OUT/$name"
cp "$TMP/read1.log" "$OUT/flashrom-read.log"
(cd "$OUT" && sha256sum "$name" > "$name.sha256")
echo "Saved $OUT/$name"
cat "$OUT/$name.sha256"
