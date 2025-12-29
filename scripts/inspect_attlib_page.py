import struct
import sys

ATTLIB_PAGE_SIZE = 2048
ATTLIB_WORDS_PER_PAGE = 512
ATTLIB_DATA_REGION_START = 0x1000

def read_page(f, page_num):
    offset = ATTLIB_DATA_REGION_START + page_num * ATTLIB_PAGE_SIZE
    f.seek(offset)
    data = f.read(ATTLIB_PAGE_SIZE)
    if len(data) < ATTLIB_PAGE_SIZE: return []
    return list(struct.unpack(f'>{ATTLIB_WORDS_PER_PAGE}I', data))

def inspect_page(file_path, page_num, target_hash):
    with open(file_path, 'rb') as f:
        words = read_page(f, page_num)
        print(f"Inspecting Page {page_num} for Hash {target_hash}...")
        for i, word in enumerate(words):
            if word == target_hash:
                print(f"Found Hash {target_hash} at Index {i}")
                # Dump context
                start = max(0, i-5)
                end = min(len(words), i+30)
                context = words[start:end]
                print("Context (Hex):")
                print([hex(w) for w in context])
                
                # Try to decode potential strings
                chars = []
                for w in context:
                    # Check if word looks like packed ASCII
                    # 4 chars per word? Or 1 char per word?
                    # E3D usually packs 4 chars in Big Endian int?
                    try:
                        packed = struct.pack('>I', w)
                        s = "".join([chr(b) if 32 <= b <= 126 else '.' for b in packed])
                        chars.append(s)
                    except:
                        chars.append("....")
                print("Context (ASCII):", chars)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: inspect_attlib_page.py <attlib.dat> <page_num> <target_hash>")
    else:
        inspect_page(sys.argv[1], int(sys.argv[2]), int(sys.argv[3]))
