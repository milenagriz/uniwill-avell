#!/usr/bin/env bash
# Runs the read-only analysis against the committed backup image and compares its output with the
# stored expected result (evidence/bios/analysis.md). Exit 0 when they match.
set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)
IMG=$REPO/evidence/bios/bios-N.1.05AVE04-20261115.bin
(cd "$REPO/evidence/bios" && sha256sum -c --quiet bios-N.1.05AVE04-20261115.bin.sha256)
out=$(mktemp); trap 'rm -f "$out"' EXIT
python3 -I "$REPO/bios/analyze.py" "$IMG" > "$out"
if diff -u "$REPO/evidence/bios/analysis.md" "$out"; then echo PASS; else echo "FAIL: output differs from evidence/bios/analysis.md"; exit 1; fi
