
import struct
import sys

ATTLIB_WORDS_PER_PAGE = 512
path = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"

with open(path, 'rb') as f:
    data = f.read()
    words = list(struct.unpack(f'>{len(data)//4}I', data))

# S2: Page 1415
s2_idx = 1415 * 512
print(f"--- Segment 2 (Page 1415) Start ---")
print(words[s2_idx:s2_idx+20])

# S6: Page 1923
s6_idx = 1923 * 512
print(f"--- Segment 6 (Page 1923) Start ---")
print(words[s6_idx:s6_idx+55]) # See up to index ~55 to see ELBO

# Check ATNAIN (S3, Page 1433) for NounID 50
s3_idx = 1433 * 512
print(f"--- Searching ATNAIN for NounID 50 ---")
# Triples: AttrHash, NounID, Offset
count = 0
for i in range(s3_idx, s3_idx + 10000, 3):
    if i+2 >= len(words): break
    attr = words[i]
    if attr == 0xFFFFFFFF: break
    noun_id = words[i+1]
    offset = words[i+2]
    
    if noun_id == 907:
        print(f"  Attr: {noun_id} -> {attr} (Offset {offset})")
        count += 1
print(f"Found {count} bindings for NounID 907")

# Check Pointer 804988 (ELBO?)
ptr_elbo = 804988
print(f"--- Content at Pointer {ptr_elbo} (ELBO) ---")
print(words[ptr_elbo:ptr_elbo+50])

# Check Pointer 896840 (PIPE?)
ptr_pipe = 896840
print(f"--- Content at Pointer {ptr_pipe} (PIPE) ---")
print(words[ptr_pipe:ptr_pipe+50])
