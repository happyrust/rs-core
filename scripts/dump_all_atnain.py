
import struct
import sys

path = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"

def decode_base27_be(hash_val):
    BASE27_OFFSET = 0x81BF1
    if hash_val < 531442: return f"unk_{hash_val}"
    k = hash_val - BASE27_OFFSET
    chars = []
    while k > 0:
        c = k % 27
        if c == 0: chars.append(' ')
        else: chars.append(chr(c + 64))
        k //= 27
    # BE Hash -> String is Reversed LE
    # So reverse it back
    return "".join(chars)[::-1]

with open(path, 'rb') as f:
    data = f.read()
    words = list(struct.unpack(f'>{len(data)//4}I', data))

s3_start = 1433 * 512
noun_attrs = {}

curr = s3_start
while curr + 2 < len(words):
    attr = words[curr]
    if attr == 0xFFFFFFFF: break
    noun_id = words[curr+1]
    offset = words[curr+2]
    
    if noun_id not in noun_attrs:
        noun_attrs[noun_id] = []
    noun_attrs[noun_id].append((attr, offset))
    curr += 3

print(f"Loaded {len(noun_attrs)} nouns from ATNAIN.")

with open("/Volumes/DPC/work/plant-code/rs-core/scripts/atnain_dump.txt", "w") as f:
    for nid, attrs in noun_attrs.items():
        attr_strs = []
        for a, off in attrs:
            name = decode_base27_be(a)
            attr_strs.append(f"{name}({a})@{off}")
        f.write(f"ID {nid}: {', '.join(attr_strs)}\n")

print("Dumped to atnain_dump.txt")
