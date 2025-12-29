
import struct
import sys

ATTLIB_WORDS_PER_PAGE = 512
path = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"

def decode_base27(hash_val):
    BASE27_OFFSET = 0x81BF1
    if hash_val < 531442: return f"unk_{hash_val}"
    k = hash_val - BASE27_OFFSET
    chars = []
    # Little Endian Decoding (Reversed String) -> Result "NAME"
    # But for Big Endian Hash?
    # Let's try to just use standard LE decoding which worked for NAME.
    # But maybe the Hash is BE.
    # If Hash is BE, then LE decoding produces Reversed String?
    # 859896 (PRO BE) -> Decoded as " ORP". Reversed "PRO ".
    
    while k > 0:
        c = k % 27
        if c == 0: chars.append(' ')
        else: chars.append(chr(c + 64))
        k //= 27
    return "".join(chars)

with open(path, 'rb') as f:
    data = f.read()
    words = list(struct.unpack(f'>{len(data)//4}I', data))

# ATNAIN Segment 3 (1433)
s3_start = 1433 * 512

noun_attrs = {} # NounID -> Set[AttrHash]

curr = s3_start
while curr + 2 < len(words):
    attr = words[curr]
    if attr == 0xFFFFFFFF: break
    noun_id = words[curr+1]
    
    if noun_id not in noun_attrs:
        noun_attrs[noun_id] = set()
    noun_attrs[noun_id].add(attr)
    curr += 3

print(f"Found {len(noun_attrs)} Nouns in ATNAIN.")

# Filter for Nouns that have QANG(867166), QRAD(879205), QPAR(877761), QARR(867285), QLEA(874936)
target_hashes = [867166, 879205, 877761, 867285, 874936]
candidates = []
for nid, attrs in noun_attrs.items():
    # If it has at least 1 of these (relaxed filter)
    for h in target_hashes:
        if h in attrs: 
            candidates.append(nid)
            break
    
print(f"Found {len(candidates)} Strong Piping Candidates.")

# Print details for candidates
print("--- Candidates ---")
for nid in candidates:
    attrs = noun_attrs[nid]
    decoded_names = []
    for a in attrs:
        d = decode_base27(a)
        d_rev = d[::-1]
        decoded_names.append(f"{d_rev}({a})")
    
    print(f"ID {nid}: {', '.join(sorted(decoded_names))}")
