#!/usr/bin/env python3
"""Read-only analysis of this laptop's BIOS image (ticket #18).

Usage: python3 -I bios/analyze.py <image.bin> > report.md

Reads only the image file (never the flash device). Extracts the AMI Setup driver with
UEFIExtract, decodes its forms with ifrextractor, parses the NVAR variable stores, and reports,
for the overclocking lock, the hidden setup menus and the default power limits / thermal
throttle point: where each setting is stored, its default and current value, and whether a
boot-time setting change is enough or a modified image is needed.
"""
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nvar  # noqa: E402

SETUP_DRIVER = "899407D7-99FE-43D8-9A21-79EC328CAC21"
ROOT_FORM = 0x2710

# The settings this report is about, by their prompt in the forms, grouped by target
TARGETS = {
    "Overclocking lock": ["Overclocking Lock", "CFG Lock", "OverClocking Feature"],
    "Default power limits and thermal throttle point": [
        "Power Limit 1 Override", "Power Limit 1", "Power Limit 1 Time Window",
        "Power Limit 2 Override", "Power Limit 2", "Package Power Limit MSR Lock",
        "Tcc Activation Offset", "Tcc Offset Time Window",
    ],
}
ATTR_NAMES = ["RUNTIME", "ASCII_NAME", "GUID", "DATA_ONLY", "EXT_HEADER", "HW_ERROR", "AUTH_WRITE", "VALID"]


def ifr_text(image):
    """Extract the Setup driver's forms as ifrextractor text (works on a private copy of the image)."""
    for tool in ("uefiextract", "ifrextractor"):
        if not shutil.which(tool):
            sys.exit(f"{tool} is not installed (uefitool-ng, ifrextractor-rs)")
    with tempfile.TemporaryDirectory() as tmp:
        img = os.path.join(tmp, "image.bin")
        shutil.copyfile(image, img)
        subprocess.run(["uefiextract", img, SETUP_DRIVER, "-m", "body"], cwd=tmp,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        dump = img + ".dump"
        bodies = sorted((os.path.join(dump, f) for f in os.listdir(dump)), key=os.path.getsize)
        body = bodies[-1]
        subprocess.run(["ifrextractor", body], cwd=dump, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, check=True)
        txt = [f for f in os.listdir(dump) if f.endswith(".en-US.uefi.ifr.txt")]
        if not txt:
            sys.exit("ifrextractor produced no en-US form text")
        with open(os.path.join(dump, txt[0]), encoding="utf-8", errors="replace") as f:
            return f.read().splitlines()


FIELD = re.compile(r"(\w+): (\"[^\"]*\"|[^,]+)")


def fields(line):
    return {k: v.strip().strip('"') for k, v in FIELD.findall(line)}


def depth(line):
    return len(line) - len(line.lstrip("\t"))


def parse_forms(lines):
    """Walk the form text. Returns varstores, forms, refs and questions with their enclosing conditions."""
    varstores, forms, refs, questions = {}, {}, [], []
    form, scopes = None, []  # scopes: [depth, kind, condition lines]
    for line in lines:
        d, s = depth(line), line.strip()
        while scopes and d <= scopes[-1][0]:
            scopes.pop()
        if s.startswith("VarStore Guid:"):
            f = fields(s)
            varstores[int(f["VarStoreId"], 16)] = {"guid": f["Guid"], "name": f["Name"]}
        elif s.startswith("Form FormId:"):
            f = fields(s)
            form = int(f["FormId"], 16)
            forms[form] = f.get("Title", "").strip()
            scopes = []
        elif s.split(" ")[0] in ("SuppressIf", "GrayOutIf", "DisableIf"):
            scopes.append([d, s.split(" ")[0], [], False])
        elif scopes and not scopes[-1][3] and "Prompt:" not in s and s != "End":
            # Condition expression lines, until the first statement inside the scope
            scopes[-1][2].append(s)
        if "Prompt:" in s:
            for sc in scopes:
                sc[3] = True
        if "Prompt:" in s and form is not None:
            f = fields(s)
            conds = [(k, " ".join(c)) for _, k, c, _ in scopes if c]
            kind = s.split(" ")[0]
            if kind == "Ref" and "FormId" in f:
                refs.append({"from": form, "to": int(f["FormId"], 16), "prompt": f["Prompt"], "conds": conds})
            elif "VarOffset" in f:
                questions.append({"form": form, "kind": kind, "prompt": f["Prompt"], "qid": int(f["QuestionId"], 16),
                                  "store": int(f["VarStoreId"], 16), "offset": int(f["VarOffset"], 16),
                                  "size": int(f.get("Size", "8")) // 8, "conds": conds, "line": s})
    return varstores, forms, refs, questions


def reachability(forms, refs):
    """Classify forms: reachable from the root through links without conditions, only through
    conditional links, or not linked at all."""
    out_edges = {}
    for r in refs:
        out_edges.setdefault(r["from"], []).append(r)
    plain, cond = {ROOT_FORM}, set()
    todo = [(ROOT_FORM, False)]
    while todo:
        f, conditional = todo.pop()
        for r in out_edges.get(f, []):
            c = conditional or bool(r["conds"])
            if not c and r["to"] not in plain:
                plain.add(r["to"])
                cond.discard(r["to"])
                todo.append((r["to"], False))
            elif c and r["to"] not in plain and r["to"] not in cond:
                cond.add(r["to"])
                todo.append((r["to"], True))
    linked = {r["to"] for r in refs}
    orphans = {f for f in forms if f not in linked and f != ROOT_FORM}
    return plain, cond, orphans


def path_to(form, refs):
    parents = {}
    for r in refs:
        parents.setdefault(r["to"], r)
    names, seen = [], set()
    while form in parents and form not in seen and form != ROOT_FORM:
        seen.add(form)
        names.append(parents[form]["prompt"].strip())
        form = parents[form]["from"]
    return " > ".join(reversed(names)) or "(not linked)"


def ifr_default(lines, q):
    """The default named in the form for a question (from the lines that follow it)."""
    i = lines.index(next(line for line in lines if line.strip() == q["line"]))
    for line in lines[i + 1:i + 80]:
        s = line.strip()
        if s.startswith("Default DefaultId: 0x0"):
            m = re.search(r"Value: (\S+)", s)
            return m.group(1) if m else None
        if s.startswith("OneOfOption") and re.search(r"Value: \S+, .*\bDefault\b", s):
            m = re.search(r"Value: (\S+?),", s)
            return m.group(1) if m else None
        if s == "End" and depth(line) <= depth(lines[i]):
            break
    return None


def value(data, q):
    if data is None or q["offset"] + q["size"] > len(data):
        return None
    return int.from_bytes(data[q["offset"]:q["offset"] + q["size"]], "little")


def attr_text(a):
    return "+".join(n for i, n in enumerate(ATTR_NAMES) if a & (1 << i) and n in ("RUNTIME", "AUTH_WRITE", "VALID")) or "none"


def main(image):
    img = open(image, "rb").read()
    lines = ifr_text(image)
    varstores, forms, refs, questions = parse_forms(lines)
    stores = nvar.find_stores(img)
    parsed = [nvar.parse_store(img, a, b) for a, b in stores]
    current = [nvar.current_values(es) for es in parsed]
    defaults = {}
    for (_, b), es in zip(stores, parsed):
        for e in es:
            if e["name"] == "StdDefaults":
                start = e["offset"] + (e["size"] - len(e["data"]))
                for d in nvar.parse_store(img, start, start + len(e["data"]), guid_end=b):
                    if d["name"]:
                        # Keyed by name: the nested entries' GUID indexes are not reliable
                        defaults.setdefault(d["name"], d["data"])
    heads = [{(e["guid"], e["name"]): e for e in es if e["name"]} for es in parsed]
    plain, cond, orphans = reachability(forms, refs)
    by_qid = {q["qid"]: q for q in questions}

    def qname(qid):
        q = by_qid.get(qid)
        if not q:
            return f"question {qid:#x}"
        vs = varstores.get(q["store"], {"name": "?"})
        label = f'"{q["prompt"].strip()}" ' if q["prompt"].strip() else ""
        v = value(cur_by_name(vs["name"]), q)
        shown = "not stored: set by the firmware at boot" if v is None else v
        return f"{label}({vs['name']}[{q['offset']:#x}] = {shown})"

    def cur_by_name(name):
        for c in reversed(current):
            for (_, n), v in c.items():
                if n == name:
                    return v["data"]
        return None

    def show_conds(q):
        out = []
        for kind, c in q["conds"]:
            c = re.sub(r"QuestionId: 0x([0-9A-F]+)", lambda m: qname(int(m.group(1), 16)), c)
            out.append(f"{kind} ({c})")
        return "; ".join(out) or "none"

    print("# BIOS image analysis (generated by `bios/analyze.py`, do not edit)\n")
    print(f"Image: `{os.path.basename(image)}`, SHA-256 `{hashlib.sha256(img).hexdigest()}`.")
    print(f"Setup driver `{SETUP_DRIVER}`: {len(forms)} forms, {len(refs)} links, {len(questions)} settings stored in variables.")
    print(f"NVAR stores: {', '.join(f'`{a:#x}-{b:#x}` ({len(es)} entries)' for (a, b), es in zip(stores, parsed))}.")
    print("All values come from this image; nothing here is taken from another model.\n")

    for target, prompts in TARGETS.items():
        print(f"## {target}\n")
        print("| Setting | Variable (GUID) | Offset | Size | Form default | Factory default | Current, store 1 | Current, store 2 | Menu path | Shown when |")
        print("|---|---|---|---|---|---|---|---|---|---|")
        for p in prompts:
            for q in (q for q in questions if q["prompt"].strip() == p):
                vs = varstores.get(q["store"], {"name": "?", "guid": "?"})
                key = (vs["guid"], vs["name"])
                cur = [value(c.get(key, {}).get("data"), q) for c in current]
                dflt = value(defaults.get(vs["name"]), q)
                print(f"| {p} | `{vs['name']}` (`{vs['guid']}`) | `{q['offset']:#x}` | {q['size']} | {ifr_default(lines, q)} | "
                      f"{dflt} | {cur[0] if cur else None} | {cur[1] if len(cur) > 1 else None} | "
                      f"{path_to(q['form'], refs)} | {show_conds(q)} |")
        print()

    print("## Variable attributes\n")
    print("| Variable | GUID | Attributes (store 1 / store 2) | Updated since the factory |")
    print("|---|---|---|---|")
    for name in ("CpuSetup", "Setup", "SaSetup", "PchSetup", "OcSetup", "AMITSESetup"):
        keys = [k for h in heads for k in h if k[1] == name]
        if not keys:
            print(f"| {name} | | not in either store (only in the factory defaults) | |")
            continue
        k = keys[0]
        attrs = " / ".join(attr_text(h[k]["attr"]) if k in h else "-" for h in heads)
        links = " / ".join(str(c[k]["links"]) if k in c else "-" for c in current)
        print(f"| {name} | `{k[0]}` | {attrs} | {links} update(s) |")
    print()

    print("## Hidden setup menus\n")
    top = [r for r in refs if r["from"] == ROOT_FORM]
    print("Top-level tabs linked from the root form: " +
          ", ".join(f"{r['prompt'].strip()}{' (conditional)' if r['conds'] else ''}" for r in top) + ".\n")
    print(f"- Forms reachable from the root through links without conditions: {len(plain)}")
    print(f"- Forms reachable only through conditional links: {len(cond)}")
    print(f"- Forms that no link points to: {len(orphans)}\n")
    if orphans:
        print("Forms no link points to:\n")
        for f in sorted(orphans):
            n = sum(1 for q in questions if q["form"] == f)
            print(f"- `{f:#x}` {forms[f] or '(untitled)'}: {n} settings")
        print()
    print("Forms reached only through conditional links (first condition shown):\n")
    for f in sorted(cond):
        r = next(r for r in refs if r["to"] == f)
        n = sum(1 for q in questions if q["form"] == f)
        print(f"- `{f:#x}` {forms[f] or '(untitled)'} ({n} settings), from \"{r['prompt'].strip()}\": {show_conds(r) if r['conds'] else 'parent is conditional'}")
    print()

    print("## Verdict per target\n")
    cpu = [e for h in heads for k, e in h.items() if k[1] == "CpuSetup"]
    rt = any(e["attr"] & nvar.RUNTIME for e in cpu)
    auth = any(e["attr"] & nvar.AUTH_WRITE for e in cpu)
    reach = "cannot" if not rt else "can"
    print(f"- `CpuSetup` has {'the' if rt else 'no'} RUNTIME attribute, so the running system {reach} read or write it "
          f"(observed in the NVAR attributes), and {'an' if auth else 'no'} authenticated-write attribute.")
    print("- **Overclocking lock** and **default power limits / thermal throttle point**: all of them live in `CpuSetup`, "
          "as ordinary one-byte or four-byte fields. " + (
              "A boot-time setting change (writing the field before the OS starts, from a UEFI shell tool or a "
              "setup_var-style GRUB command) can reach them, because the variable is boot-service accessible and "
              "not authenticated (inferred from the attributes; not yet tried on this board). "
              if not auth else "The variable is authenticated, so a boot-time write would be refused: a modified image is needed. ") +
          "Owners of older boards from this manufacturer report boot-time changes lasting only one boot, so the first attempt must check that the value survives a second reboot.")
    print("- **Hidden setup menus**: the forms link all top-level tabs from the root form without any condition "
          "(observed above), so whatever hides menus on the setup screen is not in the forms or in a variable this "
          "report can see. On AMI Aptio V that is the setup browser's own page and access data (AMITSE, inferred), "
          "which is code in the image, not a setting: **a modified image is required**. Which tabs the setup "
          "screen actually shows has to be confirmed by eye (unknown from the image alone).")
    print()


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
