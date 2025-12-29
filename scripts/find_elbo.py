
import json
import sys

def decode_base27(hash_val):
    BASE27_OFFSET = 0x81BF1
    if hash_val < 531442: return f"unk_{hash_val}"
    k = hash_val - BASE27_OFFSET
    chars = []
    while k > 0:
        c = k % 27
        if c == 0: chars.append(' ')
        else: chars.append(chr(c + 64))
        k //= 27
    return "".join(chars)

path = "/Volumes/DPC/work/plant-code/rs-core/all_attr_info_v3.json"
with open(path, 'r') as f:
    data = json.load(f)

found = False
for h, attrs in data.items():
    try:
        h_int = int(h)
        name = decode_base27(h_int)
        if "ELBO" in name or "PIPE" in name or "VALV" in name:
            print(f"Found {name}: Hash {h}")
            if "ELBO" in name:
                print(f"Attributes for {name} ({h}):")
                for attr_h, attr_info in attrs.items():
                    print(f"  {attr_info.get('name')} ({attr_h}): Offset {attr_info.get('offset')}")
                found = True
    except:
        pass

if not found:
    print("ELBO not found via decoding.")
