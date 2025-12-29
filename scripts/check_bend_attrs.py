
import json

path = "/Volumes/DPC/work/plant-code/rs-core/all_attr_info_v3.json"
with open(path, 'r') as f:
    data = json.load(f)

if "noun_attr_info_map" in data:
    data = data["noun_attr_info_map"]

targets = {
    "ELBO": "828473",
    "PIPE": "641779",
    "BEND": "620516"
}

for name, h in targets.items():
    if h in data:
        attrs = data[h]
        print(f"{name} ({h}) Attributes ({len(attrs)}):")
        cnt = 0
        for ah, info in attrs.items():
            if cnt < 5:
                print(f"  {info.get('name')} ({ah}): Offset {info.get('offset')}")
            cnt += 1
        if cnt >= 5: print("  ...")
    else:
        print(f"{name} ({h}) not found.")
