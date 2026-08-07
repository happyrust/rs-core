"""Emit the compact noun_flags.json that rs-core embeds.

Boolean columns store only the minority set, integer columns only the non-default
entries; the full table is 464 KB of JSON, this form is an order of magnitude smaller.
"""

import json
from collections import Counter

BOOLS = ["PRMF", "GORP", "XTRF", "POPF", "VISI", "PICK", "TOPF",
         "VOLDEF", "CLWTHN", "CLRSEC"]
INTS = ["GRAPH", "IMAP", "SECOND"]

rows = json.load(open("noun_flags.json", encoding="utf-8"))

out = {"nouns": [], "bools": {}, "ints": {}}
for r in rows:
    out["nouns"].append(r["hash"])

for flag in BOOLS:
    # attlib encodes logical fields as 1 = true, 2 = false
    truth = {r["hash"]: r[flag] == 1 for r in rows}
    majority = Counter(truth.values()).most_common(1)[0][0]
    exceptions = sorted(h for h, v in truth.items() if v != majority)
    out["bools"][flag] = {"default": majority, "exceptions": exceptions}

for flag in INTS:
    values = {r["hash"]: r[flag] for r in rows if isinstance(r[flag], int)}
    majority = Counter(values.values()).most_common(1)[0][0]
    entries = sorted((h, v) for h, v in values.items() if v != majority)
    out["ints"][flag] = {"default": majority, "values": entries}

with open("noun_flags_compact.json", "w", encoding="utf-8") as fh:
    json.dump(out, fh, separators=(",", ":"))

print(f"nouns={len(out['nouns'])}")
for flag, d in out["bools"].items():
    print(f"  bool {flag:7s} default={d['default']!s:5s} exceptions={len(d['exceptions'])}")
for flag, d in out["ints"].items():
    print(f"  int  {flag:7s} default={d['default']} non-default={len(d['values'])}")
