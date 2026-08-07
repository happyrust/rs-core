"""把 PRMF 缺口按专业分组，并追出手写表里那些查无此 noun 的名字来自哪张表。"""
import json
import re
from collections import defaultdict

SRC = r"D:\work\plant-code\rs-core\src\pdms_types.rs"

flags = {r["noun"]: r for r in json.load(open("noun_flags.json", encoding="utf-8"))}
graph = json.load(open("noun_graph_full.json", encoding="utf-8"))
prm = {n for n, r in flags.items() if r["PRMF"] == 1}

src = open(SRC, encoding="utf-8").read()
tables = {}
for m in re.finditer(r"(?m)^\s*pub (?:static|const)\s+([A-Z0-9_]+)\s*:[^=]*=\s*\[(.*?)\];", src, re.S):
    tables[m.group(1)] = [x.group(1) for x in re.finditer(r'"([A-Z0-9]+)"', m.group(2))]

covered = set().union(*tables.values()) if tables else set()
missing = sorted(prm - covered)

# 手写表里字典查无此 noun 的名字 -> 追回来源表
unknown = defaultdict(list)
for t, names in tables.items():
    for n in names:
        if n not in flags:
            unknown[n].append(t)

# 专业前缀分组：只是启发式，用来判断缺口是否落在本项目处理范围内
BUCKETS = [
    ("船体 / Hull", lambda n: n.startswith(("H", "I")) and n not in ("HVAC",)),
    ("电缆桥架 CT*", lambda n: n.startswith("CT")),
    ("HVAC", lambda n: n.startswith("HV")),
    ("辅助元素 AID*", lambda n: n.startswith("AID")),
    ("目录/形状 A* 前缀", lambda n: n.startswith("A") and len(n) >= 4),
    ("制图 / 标注", lambda n: n.startswith(("DIM", "DRAW", "MLABEL", "LINDIM"))),
    ("建筑 / 房间", lambda n: n in {"DOOR", "WINDOW", "WLPANE", "WLPROF", "WLOPEN", "WLJOIN",
                                    "WLFEAT", "FLRCOV", "FLRLAY", "CEILIN", "KICKPL", "PLTFRM"}),
]


def bucket(n):
    for name, pred in BUCKETS:
        if pred(n):
            return name
    return "其他"


by_bucket = defaultdict(list)
for n in missing:
    by_bucket[bucket(n)].append(n)

print(f"pdms_types.rs 常量表 {len(tables)} 张，并集 {len(covered)} 个名字")
print(f"PRMF=true {len(prm)}，未被任何表覆盖 {len(missing)}\n")

print("=== 缺口按专业分组 ===")
for b, ns in sorted(by_bucket.items(), key=lambda kv: -len(kv[1])):
    in_design = sum(1 for n in ns if n in graph)
    print(f"  {b:22s} {len(ns):4d} 个（其中 {in_design} 个在 desvir 设计模板库里）")

print("\n=== 手写表里字典查无此 noun 的名字，来自哪张表 ===")
for n, ts in sorted(unknown.items()):
    print(f"  {n:8s} <- {', '.join(ts)}")

print("\n=== 「其他」桶明细（最需要人工判断的部分）===")
other = by_bucket.get("其他", [])
for i in range(0, len(other), 12):
    print("  " + " ".join(other[i:i + 12]))
