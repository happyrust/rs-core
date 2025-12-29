
import json

def decode_base27(hash_val):
    BASE27_OFFSET = 0x81BF1
    if hash_val < 531442: return f"unk_{hash_val}"
    k = hash_val - BASE27_OFFSET
    chars = []
    # 假设它是4字符
    # 像 NAME (639374 -> 107933) -> N(14), A(1), M(13), E(5)
    # 14 + 1*27 ...
    # k % 27 -> char 1
    while k > 0:
        c = k % 27
        if c == 0: chars.append(' ')
        else: chars.append(chr(c + 64))
        k //= 27
    return "".join(chars)

path = "/Volumes/DPC/work/plant-code/rs-core/all_attr_info_v3.json"
with open(path, 'r') as f:
    data = json.load(f)

if "noun_attr_info_map" in data:
    data = data["noun_attr_info_map"]

print(f"Total Nouns: {len(data)}")
for h in data.keys():
    try:
        h_int = int(h)
        name = decode_base27(h_int)
        print(f"{h}: {name}")
    except:
        print(f"{h}: Decode error")

# Also check specific candidates
# PIPE 
# ELBO
