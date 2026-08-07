"""Dump the E3D noun classification table out of attlib.dat.

Reimplements core.dll's ATTOPE / ATGTIX / ATGTDF / ATRDRC / ATNLOG chain.
"""

import json
import struct
import sys

PAGE_WORDS = 512
PAGE_BYTES = PAGE_WORDS * 4
DIR_PAGE = 2
MIN_HASH = 531442
MAX_HASH = 387951929
SYNO = 837586  # inheritance link field ATNLOG falls back to

BASE27 = 0x81BF1


def dehash(value):
    if not BASE27 < value <= 0x171FAD39:
        return None
    value -= BASE27
    out = []
    while value:
        value, digit = divmod(value, 27)
        out.append(" " if digit == 0 else chr(64 + digit))
    return "".join(out)


def rehash(name):
    val = 0
    for ch in reversed(name.upper()):
        val = val * 27 + (0 if ch == " " else ord(ch) - 64)
    return val + BASE27


class Attlib:
    def __init__(self, path, endian=">", page_base=1):
        self.blob = open(path, "rb").read()
        self.endian = endian
        self.page_base = page_base

    def page(self, n):
        off = (n - self.page_base) * PAGE_BYTES
        if off < 0 or off + PAGE_BYTES > len(self.blob):
            raise IndexError(f"page {n} out of range")
        return list(struct.unpack(f"{self.endian}512i", self.blob[off:off + PAGE_BYTES]))

    def load_index(self, start_page):
        """ATGTIX: (hash, combined) pairs; combined = page*512 + offset."""
        out = {}
        pg = start_page
        while True:
            words = self.page(pg)
            i = 0
            while i < PAGE_WORDS:
                w = words[i]
                if not (MIN_HASH <= w <= MAX_HASH):
                    break
                combined = words[i + 1]
                out[w] = (combined // 512, combined % 512)
                i += 2
            w = words[i] if i < PAGE_WORDS else 0
            if w == 0:
                pg += 1
                continue
            if w == -1:
                return out
            raise ValueError(f"bad ATGTIX word {w} at page {pg} idx {i}")

    def load_defs(self, start_page):
        """ATGTDF: (hash, dataType, flag[, default...])."""
        out = {}
        pg = start_page
        while True:
            words = self.page(pg)
            i = 0
            while i < PAGE_WORDS:
                w = words[i]
                if not (MIN_HASH <= w <= MAX_HASH):
                    break
                dtype = words[i + 1]
                flag = words[i + 2]
                i += 3
                default = None
                if flag == 2:
                    if dtype == 4:
                        n = words[i]
                        default = words[i + 1:i + 1 + n]
                        i += 1 + n
                    else:
                        default = words[i]
                        i += 1
                elif flag != 1:
                    raise ValueError(f"bad ATGTDF flag {flag}")
                out[w] = {"type": dtype, "default": default}
            w = words[i] if i < PAGE_WORDS else 0
            if w == 0:
                pg += 1
                continue
            if w == -1:
                return out
            raise ValueError(f"bad ATGTDF word {w} at page {pg} idx {i}")


class NounDict:
    def __init__(self, lib, noun_index, field_defs, field_order):
        self.lib = lib
        self.noun_index = noun_index
        self.field_defs = field_defs
        self.field_order = field_order  # hash -> 1-based column
        self._pages = {}

    def _page(self, n):
        if n not in self._pages:
            self._pages[n] = self.lib.page(n)
        return self._pages[n]

    def lookup(self, noun_hash, field_hash, _depth=0):
        """ATNLOG: returns (value, source) or (None, reason)."""
        if _depth > 16:
            return None, "syno-loop"
        col = self.field_order.get(field_hash)
        if col is None:
            return None, "no-such-field"
        entry = self.noun_index.get(noun_hash)
        if entry is None:
            return None, "no-such-noun"
        pgno, off = entry
        page = self._page(pgno)
        slot = page[off + col - 2]
        if slot == 0:
            syno_col = self.field_order.get(SYNO)
            if syno_col is None:
                return None, "no-syno"
            syno_slot = page[off + syno_col - 2]
            if syno_slot in (0, -1):
                return None, "unset"
            parent = page[off + syno_slot - 2]
            return self.lookup(parent, field_hash, _depth + 1)
        if slot == -1:
            return self.field_defs[field_hash]["default"], "default"
        return page[off + slot - 2], "own"


FLAGS = ["PRMF", "GORP", "XTRF", "POPF", "GRAPH", "IMAP", "VISI",
         "PICK", "TOPF", "VOLDEF", "CLWTHN", "CLRSEC", "SECOND"]


def main(path):
    for endian in (">", "<"):
        for base in (1, 0):
            try:
                lib = Attlib(path, endian, base)
                seg = lib.page(DIR_PAGE)[:8]
                if not all(0 < s < len(lib.blob) // PAGE_BYTES for s in seg):
                    continue
                noun_index = lib.load_index(seg[6])
                field_defs = lib.load_defs(seg[4])
            except Exception:
                continue
            if len(noun_index) > 100 and rehash("PRMF") in field_defs:
                print(f"endian={endian!r} page_base={base} segments={seg}")
                print(f"nouns={len(noun_index)} fields={len(field_defs)}")
                return report(lib, seg, noun_index, field_defs)
    print("could not lock onto a layout", file=sys.stderr)
    return 1


def report(lib, seg, noun_index, field_defs):
    order = {h: i + 1 for i, h in enumerate(field_defs)}
    nd = NounDict(lib, noun_index, field_defs, order)

    print("\n-- field columns --")
    for h, i in order.items():
        d = field_defs[h]
        print(f"  {i:3d} {dehash(h) or h!s:8s} type={d['type']} default={d['default']}")

    rows = []
    for nh in noun_index:
        name = dehash(nh)
        if not name:
            continue
        row = {"noun": name.strip(), "hash": nh}
        for f in FLAGS:
            v, src = nd.lookup(nh, rehash(f))
            row[f] = v
        rows.append(row)

    geo = [r for r in rows if r.get("PRMF") in (1, -1)]
    print(f"\n-- nouns with PRMF true: {len(geo)} / {len(rows)} --")
    print("  " + " ".join(sorted(r["noun"] for r in geo)))

    with open("noun_flags.json", "w", encoding="utf-8") as fh:
        json.dump(rows, fh, indent=1, ensure_ascii=False)
    print(f"\nwrote noun_flags.json ({len(rows)} nouns)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1
                  else r"D:\AVEVA\Everything3D3.1\attlib.dat"))
