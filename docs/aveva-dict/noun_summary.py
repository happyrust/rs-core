import json
from collections import Counter

rows = json.load(open("noun_flags.json", encoding="utf-8"))
by_name = {r["noun"]: r for r in rows}

BOOL = ["PRMF", "GORP", "XTRF", "POPF", "VISI", "PICK", "TOPF",
        "VOLDEF", "CLWTHN", "CLRSEC"]
INT = ["GRAPH", "IMAP", "SECOND"]

print(f"nouns: {len(rows)}\n")
print("-- boolean flags (1=true, 2=false) --")
for f in BOOL:
    c = Counter(r[f] for r in rows)
    print(f"  {f:8s} true={c[1]:5d} false={c[2]:5d} other={sum(v for k, v in c.items() if k not in (1, 2))}")

print("\n-- integer flags: value -> count --")
for f in INT:
    c = Counter(r[f] for r in rows)
    print(f"  {f:8s} " + "  ".join(f"{k}:{v}" for k, v in sorted(c.items(), key=lambda x: (x[0] is None, x[0]))))

print("\n-- GRAPH value -> sample nouns --")
buckets = {}
for r in rows:
    buckets.setdefault(r["GRAPH"], []).append(r["noun"])
for k in sorted(buckets, key=lambda x: (x is None, x)):
    names = sorted(buckets[k])
    print(f"  GRAPH={k!s:5s} n={len(names):5d}  {' '.join(names[:14])}")

print("\n-- GORP (geomset) true --")
print("  " + " ".join(sorted(r["noun"] for r in rows if r["GORP"] == 1)))

print("\n-- XTRF (extrusion) true --")
print("  " + " ".join(sorted(r["noun"] for r in rows if r["XTRF"] == 1)))

print("\n-- spot checks --")
for n in ["BOX", "CYLI", "ELBO", "TUBI", "SITE", "ZONE", "EQUI", "PIPE",
          "BRAN", "STRU", "SCTN", "GMSET", "SCOM", "WORL", "NOZZ", "PANE"]:
    r = by_name.get(n)
    if r is None:
        print(f"  {n:6s} (absent)")
        continue
    print(f"  {n:6s} PRMF={r['PRMF']} GORP={r['GORP']} XTRF={r['XTRF']} "
          f"POPF={r['POPF']} GRAPH={r['GRAPH']} IMAP={r['IMAP']} "
          f"VISI={r['VISI']} TOPF={r['TOPF']} VOLDEF={r['VOLDEF']}")
