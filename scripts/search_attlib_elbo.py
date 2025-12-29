
import struct
import sys

ATTLIB_PAGE_SIZE = 2048
ATTLIB_WORDS_PER_PAGE = 512
path = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"

# ELBO Hash candidates
# E(5), L(12), B(2), O(15)
# Base27 LE: 5 + 12*27 + 2*729 + 15*19683 = 5 + 324 + 1458 + 295245 = 297032
# Offset 0x81BF1 (531441)
# Result: 297032 + 531441 = 828473

# NAME (639374) -> 107933
# N(14), A(1), M(13), E(5)
# 14 + 1*27 + ... NO
# NAME decoded as N, A, M, E.
# 639374 - 531441 = 107933
# 107933 % 27 = 14 (N).
# So Little Endian decoding order.
# Hash construction: H = c0 + c1*27 + c2*27^2...
# So N is c0 (LSB).
# So NAME is N + A*27 + M*27^2 + E*27^3.
# N=14, A=1, M=13, E=5.
# 14 + 27 + 13*729 + 5*19683 ?
# 14 + 27 + 9477 + 98415 = 107933. CORRECT.

# So ELBO: E(5), L(12), B(2), O(15).
# E + L*27 + B*729 + O*19683
# 5 + 12*27 + 2*729 + 15*19683
# 5 + 324 + 1458 + 295245 = 297032.
# Plus offset 531441 = 828473.

hashes_to_find = [828473, 641779, 620516]

with open(path, 'rb') as f:
    data = f.read()
    words = list(struct.unpack(f'>{len(data)//4}I', data))

print(f"Searching for {hashes_to_find} in {len(words)} words...")
found_count = 0
for i, w in enumerate(words):
    if w in hashes_to_find:
        found_count += 1
        page = i // 512
        segment = -1
        # [3, 4, 1415, 1433, 1466, 1467, 1923, 1929]
        # S0: 3
        # S1: 4
        # S2: 1415
        # S3: 1433 (ATNAIN)
        # S4: 1466
        # S5: 1467
        # S6: 1923
        if page >= 1923: segment = 6
        elif page >= 1467: segment = 5
        elif page >= 1466: segment = 4
        elif page >= 1433: segment = 3
        elif page >= 1415: segment = 2
        elif page >= 4: segment = 1
        elif page >= 3: segment = 0
        
        print(f"Found {w} at word index {i} (Page {page}, Offset {i % 512}, Segment {segment})")
        # Print context
        context = words[max(0, i-10):i+20]
        print(f"Context: {context}")

if found_count == 0:
    print("Not found.")
