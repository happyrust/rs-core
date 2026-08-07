"""对比 rs-core 手写的 VISBILE_GEO_NOUNS 与 attlib 里的 PRMF/GORP 旗标。"""

import json

flags = {r["noun"]: r for r in json.load(open("noun_flags.json", encoding="utf-8"))}
cur = open("_visible_geo_nouns.txt", encoding="utf-8-sig").read().strip().split(",")
prm = {n for n, r in flags.items() if r["PRMF"] == 1}
gor = {n for n, r in flags.items() if r["GORP"] == 1}

print(f"hardcoded VISBILE_GEO_NOUNS : {len(cur)}")
print(f"PRMF=true (design geometry) : {len(prm)}")
print(f"GORP=true (catalogue geom)  : {len(gor)}")
print()

notprm = [n for n in cur if n not in prm]
print(f"hardcoded but PRMF != true  : {len(notprm)}")
for n in notprm:
    r = flags.get(n)
    if r is None:
        print(f"  {n:8s} -> 字典里根本没有这个 noun")
    else:
        print(
            f"  {n:8s} PRMF={r['PRMF']} GORP={r['GORP']} XTRF={r['XTRF']} "
            f"TOPF={r['TOPF']} VISI={r['VISI']} GRAPH={r['GRAPH']}"
        )
print()

missing = sorted(prm - set(cur))
print(f"PRMF=true but NOT hardcoded : {len(missing)}")
print("  " + " ".join(missing))
print()

neg = [n for n in missing if n.startswith("N")]
print(f"其中以 N 开头（疑似负体）    : {len(neg)}")
print(f"其余                        : {len(missing) - len(neg)}")
