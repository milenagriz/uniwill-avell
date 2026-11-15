import sys
d = open(sys.argv[1], 'rb').read()
def hexdump(lo, hi):
    for o in range(lo, hi, 16):
        row = d[o:o+16]
        if not any(row) : continue
        print(f"{o:03x}: " + " ".join(f"{b:02x}" for b in row) + "  " + "".join(chr(b) if 32 <= b < 127 else "." for b in row))
for lo, hi in [(int(a,16), int(b,16)) for a, b in (r.split('-') for r in sys.argv[2:])]:
    hexdump(lo, hi); print()
