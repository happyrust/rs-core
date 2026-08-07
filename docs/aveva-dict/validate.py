"""Cross-check the derived noun graph and prune gate against a real Dabacon model."""

import json
import struct
import sys
from collections import Counter, defaultdict

DB = r"D:\work\plant-code\pdms-io\pdms-test-data\sam7200_0001"
PAGE = 2048
BASE27 = 0x81BF1


def dehash(v):
    if not BASE27 < v <= 0x171FAD39:
        return None
    v -= BASE27
    out = []
    while v:
        v, d = divmod(v, 27)
        out.append(" " if d == 0 else chr(64 + d))
    return "".join(out).strip()


class Db:
    def __init__(self, path):
        self.blob = open(path, "rb").read()

    def page(self, n):
        off = n * PAGE
        return self.blob[off:off + PAGE]

    def header(self):
        v = struct.unpack_from(">16I", self.blob, 0)
        return {"latest_ses_pgno": v[10], "stored_pages": v[14]}

    def session(self, pgno):
        p = self.page(pgno)
        return {
            "type": struct.unpack_from(">I", p, 0)[0],
            "prev": struct.unpack_from(">I", p, 4)[0],
            "index_root": struct.unpack_from(">I", p, 0x1C)[0],
        }

    def scan_records(self, db_num):
        """Element records are word-aligned [len][ref0][ref1][type][own0][own1]."""
        out = {}
        blob = self.blob
        for pos in range(0, len(blob) - 24, 4):
            n, r0, r1, ty, o0, o1 = struct.unpack_from(">6I", blob, pos)
            if r0 != db_num or o0 != db_num:
                continue
            if not (0 < n < 2048) or r1 == 0:
                continue
            name = dehash(ty)
            if not name or not (2 <= len(name) <= 6) or not name.isalpha():
                continue
            # later copies win: the newest session's record sits further in
            out[(r0, r1)] = {"refno": (r0, r1), "type": ty, "owner": (o0, o1)}
        return out


def main():
    db = Db(DB)
    hdr = db.header()
    ses = db.session(hdr["latest_ses_pgno"])
    print(f"index root page {ses['index_root']}")

    db_num = 23584
    elements = db.scan_records(db_num)
    print(f"elements read: {len(elements)}")

    tycount = Counter()
    for e in elements.values():
        tycount[dehash(e["type"]) or hex(e["type"])] += 1
    print(f"distinct types present: {len(tycount)}")
    print("  top:", tycount.most_common(12))

    pairs = Counter()
    orphan = 0
    for e in elements.values():
        owner = elements.get(e["owner"])
        if owner is None:
            orphan += 1
            continue
        a, b = dehash(owner["type"]), dehash(e["type"])
        if a and b:
            pairs[(a, b)] += 1
    print(f"observed owner->member pairs: {len(pairs)} distinct, {orphan} elements with no in-file owner")

    graph = json.load(open("noun_graph_full.json", encoding="utf-8"))
    flags = {r["noun"]: r for r in json.load(open("noun_flags.json", encoding="utf-8"))}
    clo = json.load(open("noun_closure.json", encoding="utf-8"))
    gate = set(clo["flag_gates"]["PRMF"]["descend_gate"])

    allowed = {(a, b) for a, v in graph.items() for b in v["members"]}
    missing = sorted(p for p in pairs if p not in allowed)
    print(f"\npairs covered by derived graph: {len(pairs) - len(missing)} / {len(pairs)}")
    if missing:
        print("  NOT in graph:")
        for a, b in missing[:20]:
            in_g = (a in graph, b in graph)
            print(f"    {a:8s} -> {b:8s}  x{pairs[(a, b)]:<6d} (owner known={in_g[0]}, member known={in_g[1]})")

    # Would a PRMF walk from the DB roots actually reach every primitive element?
    prim = {n for n, r in flags.items() if r["PRMF"] == 1}
    prim_els = [e for e in elements.values() if dehash(e["type"]) in prim]
    print(f"\nprimitive elements in model: {len(prim_els)}")

    blocked = defaultdict(int)
    reached = 0
    for e in prim_els:
        cur, path_ok, guard = e, True, 0
        while guard < 200:
            guard += 1
            owner = elements.get(cur["owner"])
            if owner is None:
                break
            name = dehash(owner["type"])
            if name not in gate:
                blocked[name] += 1
                path_ok = False
                break
            cur = owner
        if path_ok:
            reached += 1
    print(f"reachable through the gate: {reached} / {len(prim_els)}")
    if blocked:
        print("  blocked at these owner types:", dict(sorted(blocked.items(), key=lambda x: -x[1])[:10]))


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    main()
