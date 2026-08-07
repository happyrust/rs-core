"""把 rs-core/pdms_types.rs 里所有手写 noun 常量表合起来，对照 attlib 的 PRMF 旗标。

回答一个具体问题：E3D 认定为设计侧几何基元（PRMF=true）的 347 个类型里，
有多少个在 rs-core 的任何一张手写表里都没出现过。
"""

import json
import re

SRC = r"D:\work\plant-code\rs-core\src\pdms_types.rs"

src = open(SRC, encoding="utf-8").read()
consts = re.findall(
    r"pub const (\w+)\s*:\s*\[&'static str[^\]]*\]\s*=\s*\[(.*?)\n\];",
    src,
    re.S,
)

lists = {}
for name, body in consts:
    lists[name] = [m.group(1) for m in re.finditer(r'"([A-Z0-9_]+)"', body)]

flags = {r["noun"]: r for r in json.load(open("noun_flags.json", encoding="utf-8"))}
prm = {n for n, r in flags.items() if r["PRMF"] == 1}

print(f"pdms_types.rs 里的 noun 常量表：{len(lists)} 张")
for name, items in sorted(lists.items(), key=lambda kv: -len(kv[1])):
    hit = len(set(items) & prm)
    print(f"  {name:32s} {len(items):4d} 项，其中 PRMF=true {hit}")

union = set()
for items in lists.values():
    union |= set(items)

print()
print(f"所有表并集                : {len(union)}")
print(f"PRMF=true 总数            : {len(prm)}")
print(f"PRMF=true 且已被任一表覆盖: {len(prm & union)}")

gap = sorted(prm - union)
print(f"PRMF=true 但一张表都没有  : {len(gap)}")
print()
print("  " + " ".join(gap))
print()

unknown = sorted(n for n in union if n not in flags)
print(f"手写表里字典查无此 noun   : {len(unknown)}")
print("  " + " ".join(unknown))
