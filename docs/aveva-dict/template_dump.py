"""Extract the owner/member graph from the Dabacon template DBs.

Mirrors core.dll's dab-462 path: directory entry -> chained block read of the
element-type descriptor -> desc[10]/desc[11] = members, desc[12]/desc[13] = owners.
"""

import json
import struct
import sys

BASE27 = 0x81BF1
MIN_HASH, MAX_HASH = 531442, 387951929
PAGE_WORDS = 512
DATA_WORDS = 511  # last word of a continued block is the link to the next


def dehash(v):
    if not BASE27 < v <= 0x171FAD39:
        return None
    v -= BASE27
    out = []
    while v:
        v, d = divmod(v, 27)
        out.append(" " if d == 0 else chr(64 + d))
    return "".join(out).strip()


class TemplateDb:
    def __init__(self, path, block_base):
        self.blob = open(path, "rb").read()
        self.block_base = block_base
        self.npages = len(self.blob) // (PAGE_WORDS * 4)

    def block(self, n):
        off = (n - self.block_base) * PAGE_WORDS * 4
        if off < 0 or off + PAGE_WORDS * 4 > len(self.blob):
            raise IndexError(f"block {n}")
        return struct.unpack(">512i", self.blob[off:off + PAGE_WORDS * 4])

    def directory(self):
        out = {}
        for p in range(self.npages):
            w = self.block(p + self.block_base)
            rows, prev = [], None
            for k in range(73):
                row = w[k * 7:k * 7 + 7]
                h = row[0]
                if not (MIN_HASH <= h <= MAX_HASH) or dehash(h) is None:
                    break
                if prev is not None and h <= prev:
                    break
                prev = h
                rows.append(row)
            if len(rows) >= 20:
                for row in rows:
                    out[row[0]] = row
        return out

    def read_chain(self, start, total):
        """Chained block read: 511 data words per block, word 511 links onward."""
        data, blk = [], start
        while len(data) < total:
            w = self.block(blk)
            take = min(DATA_WORDS, total - len(data))
            data.extend(w[:take])
            if len(data) >= total:
                break
            blk = w[DATA_WORDS]
        return data


def relations(db, row):
    desc = db.read_chain(row[1], row[2])
    if len(desc) < 14:
        return None
    if desc[1] != row[0]:
        return None
    nmem, omem = desc[10], desc[11]
    nown, oown = desc[12], desc[13]
    members, owners = [], []
    if 0 < nmem < 4096 and 0 < omem < len(desc):
        members = desc[omem:omem + nmem]
    if 0 < nown < 4096 and 0 < oown < len(desc):
        owners = desc[oown:oown + nown]
    return {
        "self_hash": desc[1],
        "n_attr_blocks": desc[9],
        "members": members,
        "owners": owners,
    }


def try_base(path, base):
    db = TemplateDb(path, base)
    d = db.directory()
    if not d:
        return None
    probe = [h for h in d if dehash(h) in ("SITE", "ZONE", "EQUI")]
    ok = 0
    for h in probe:
        try:
            r = relations(db, d[h])
        except IndexError:
            r = None
        if r:
            ok += 1
    return (db, d, ok) if ok == len(probe) and probe else None


def main(paths):
    graph = {}
    for path in paths:
        found = None
        for base in (0, 1):
            found = try_base(path, base)
            if found:
                print(f"{path}: block_base={base}")
                break
        if not found:
            print(f"{path}: layout not recognised", file=sys.stderr)
            continue
        db, d, _ = found
        good = bad = 0
        for h, row in d.items():
            try:
                r = relations(db, row)
            except IndexError:
                r = None
            if not r:
                bad += 1
                continue
            good += 1
            name = dehash(h)
            entry = graph.setdefault(name, {"hash": h, "members": [], "owners": [], "db": []})
            entry["db"].append(path.rsplit("\\", 1)[-1])
            for m in r["members"]:
                n = dehash(m)
                if n and n not in entry["members"]:
                    entry["members"].append(n)
            for o in r["owners"]:
                n = dehash(o)
                if n and n not in entry["owners"]:
                    entry["owners"].append(n)
        print(f"  types={len(d)} decoded={good} failed={bad}")

    edges = sum(len(v["members"]) for v in graph.values())
    print(f"\ntotal types={len(graph)}  member-edges={edges}")
    for probe in ("SITE", "ZONE", "EQUI", "PIPE", "BRAN", "STRU", "FRMW", "SCTN", "BOX"):
        v = graph.get(probe)
        if not v:
            print(f"  {probe}: absent")
            continue
        print(f"  {probe:5s} owners={v['owners']}")
        print(f"        members({len(v['members'])})={v['members'][:22]}")

    with open("noun_graph_full.json", "w", encoding="utf-8") as fh:
        json.dump(graph, fh, indent=1, ensure_ascii=False)
    print("\nwrote noun_graph_full.json")


if __name__ == "__main__":
    main(sys.argv[1:] or [r"D:\AVEVA\Everything3D3.1\desvir.dat",
                          r"D:\AVEVA\Everything3D3.1\padvir.dat"])
