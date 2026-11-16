#!/usr/bin/env python3
"""Read-only structure dump of an NVIDIA vBIOS file (e.g. evidence/gpu/vbios-rom-sysfs.bin).

Prints the PCI image chain (PCIR and NVIDIA's NPDE extension), the subsystem ID stored in the
image, the BIT token table, and where the BIT 'P' (performance) pointers point. Parsing follows
nouveau (drivers/gpu/drm/nouveau/nvkm/subdev/bios/{base,bit,image,npde,power_budget}.c).

    python -I vbios.py ../gpu/vbios-rom-sysfs.bin
"""
import struct
import sys


def main(path):
    d = open(path, "rb").read()
    print(f"file {path}: {len(d)} bytes")

    off = 0
    while d[off:off + 2] == b"\x55\xaa":
        pcir = off + struct.unpack_from("<H", d, off + 0x18)[0]
        ven, dev = struct.unpack_from("<HH", d, pcir + 4)
        pcir_size = struct.unpack_from("<H", d, pcir + 0x10)[0] * 512
        pcir_last = d[pcir + 0x15] >> 7
        npde = (pcir + struct.unpack_from("<H", d, pcir + 0xA)[0] + 0xF) & ~0xF
        line = (f"image {off:#07x}: type {d[pcir + 0x14]:#04x} ven {ven:04x} dev {dev:04x} "
                f"PCIR size {pcir_size:#x} last {pcir_last}")
        size, last = pcir_size, pcir_last
        if d[npde:npde + 4] == b"NPDE":
            size = struct.unpack_from("<H", d, npde + 8)[0] * 512
            last = d[npde + 10] >> 7
            line += f" | NPDE size {size:#x} last {last}"
        print(line)
        off += size
        if last or off >= len(d):
            break
    if off >= len(d) and not last:
        print(f"chain continues past the end of the file at {off:#x}: the extended image is missing")

    ssid = d.find(b"\x05\x1d")
    if ssid >= 0:
        sv, sd = struct.unpack_from("<HH", d, ssid)
        print(f"first 1d05 (Tongfang) subsystem ID in the image: {sv:04x}:{sd:04x} at {ssid:#x}")

    bo = d.find(b"\xff\xb8BIT")
    if bo < 0:
        print("no BIT table")
        return
    esz, n = d[bo + 9], d[bo + 10]
    print(f"BIT at {bo:#x}, {n} tokens")
    tokens = {}
    for i in range(n):
        e = bo + 12 + i * esz
        tid, ver = d[e], d[e + 1]
        ln, toff = struct.unpack_from("<HH", d, e + 2)
        tokens[chr(tid)] = (ver, ln, toff)
        print(f"  token {chr(tid)!r} v{ver} len {ln:#x} at {toff:#x}")
    if "P" in tokens:
        ver, ln, toff = tokens["P"]
        ptrs = [struct.unpack_from("<I", d, toff + i)[0] for i in range(0, ln, 4)]
        nz = [p for p in ptrs if p]
        print(f"BIT 'P' v{ver}: {len(nz)} non-zero pointers, {min(nz):#x}..{max(nz):#x}; "
              f"{sum(p >= len(d) for p in nz)} point past the end of this file")
        pb = struct.unpack_from("<I", d, toff + 0x2C)[0]
        where = "inside" if pb < len(d) else "outside"
        print(f"power budget table pointer (P+0x2c): {pb:#x}, {where} this file")


if __name__ == "__main__":
    main(sys.argv[1])
