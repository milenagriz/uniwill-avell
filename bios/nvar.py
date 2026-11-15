"""Read-only parser for AMI Aptio NVAR variable stores inside a BIOS image file.

Layout (from UEFITool's nvram.h): each entry starts with
  'NVAR' signature, u16 size (whole entry), u24 next (offset of the next entry in the variable's
  update chain, relative to this entry; 0xFFFFFF = last), u8 attributes.
Unless DATA_ONLY, a 1-byte GUID index (or a full 16-byte GUID with the GUID attribute) and the name
follow; the GUID list sits at the end of the store, growing downwards. With EXT_HEADER, the last
u16 of the entry is the size of an extended header at the end of the entry.
"""
import struct
import uuid

SIG = b"NVAR"
STORE_GUID = uuid.UUID("CEF5B9A3-476D-497F-9FDC-E98143E0422C").bytes_le
RUNTIME, ASCII_NAME, GUID_ATTR, DATA_ONLY, EXT_HEADER, HW_ERR, AUTH_WRITE, VALID = (1 << i for i in range(8))


def find_stores(img):
    """Return (start, end) of every NVAR store: the body of an FFS file with the NVAR store GUID."""
    stores, pos = [], 0
    while True:
        pos = img.find(STORE_GUID, pos)
        if pos < 0:
            return stores
        # FFS file header: GUID(16) checksum(2) type(1) attributes(1) size(3) state(1)
        size = int.from_bytes(img[pos + 20:pos + 23], "little")
        if img[pos + 24:pos + 28] == SIG and 0x100 < size < 0x1000000:
            stores.append((pos + 24, pos + size))
        pos += 16


def parse_store(img, start, end, guid_end=None):
    """Parse one store. Returns a list of entries (dicts) in storage order. guid_end is the end of
    the store whose GUID list applies (the outer store, for the entries nested in StdDefaults)."""
    guid_end = guid_end or end
    entries, off = [], start
    while off + 10 <= end and img[off:off + 4] == SIG:
        size, = struct.unpack_from("<H", img, off + 4)
        nxt = int.from_bytes(img[off + 6:off + 9], "little")
        attr = img[off + 9]
        if size < 10 or off + size > end:
            break
        body = img[off + 10:off + size]
        e = {"offset": off, "size": size, "next": None if nxt == 0xFFFFFF else off + nxt,
             "attr": attr, "valid": bool(attr & VALID), "data_only": bool(attr & DATA_ONLY),
             "guid": None, "name": None}
        if attr & EXT_HEADER and len(body) >= 2:
            ext, = struct.unpack_from("<H", body, len(body) - 2)
            if 0 < ext <= len(body):
                body = body[:len(body) - ext]
        if not e["data_only"]:
            if attr & GUID_ATTR:
                e["guid"] = str(uuid.UUID(bytes_le=bytes(body[:16]))).upper()
                body = body[16:]
            else:
                idx = body[0]
                g = guid_end - 16 * (idx + 1)
                e["guid"] = str(uuid.UUID(bytes_le=bytes(img[g:g + 16]))).upper()
                body = body[1:]
            if attr & ASCII_NAME:
                n = body.index(0)
                e["name"], body = bytes(body[:n]).decode("ascii", "replace"), body[n + 1:]
            else:
                n = 0
                while body[n:n + 2] != b"\0\0":
                    n += 2
                e["name"], body = bytes(body[:n]).decode("utf-16-le", "replace"), body[n + 2:]
        e["data"] = bytes(body)
        entries.append(e)
        off += size
    return entries


def current_values(entries):
    """Follow every named entry's update chain to its last valid link.
    Returns {(guid, name): {"head": offset, "current": offset, "data": bytes, "links": n}}."""
    by_off = {e["offset"]: e for e in entries}
    out = {}
    for e in entries:
        if e["data_only"] or e["name"] is None:
            continue
        key = (e["guid"], e["name"])
        cur, seen, links = e, set(), 0
        while cur["next"] is not None and cur["next"] in by_off and cur["next"] not in seen:
            seen.add(cur["offset"])
            cur, links = by_off[cur["next"]], links + 1
        if not e["valid"] and not cur["valid"]:
            continue
        # A later complete copy of the same variable replaces an earlier one
        out[key] = {"head": e["offset"], "current": cur["offset"], "data": cur["data"], "links": links}
    return out
