import struct

ATTLIB_PATH = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"
PAGE_SIZE = 2048
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x0800

def decode_base27(hash_val):
    BASE27_OFFSET = 0x81BF1
    if hash_val < 531442: return ""
    k = hash_val - BASE27_OFFSET
    chars = []
    while k > 0:
        c = k % 27
        if c == 0: chars.append(' ')
        else: chars.append(chr(c + 64))
        k //= 27
    return "".join(chars)

def analyze():
    with open(ATTLIB_PATH, "rb") as f:
        # 段 2 起始页
        f.seek(SEGMENT_POINTERS_OFFSET)
        ptrs = struct.unpack(">8I", f.read(32))
        start_page = ptrs[2]
        print(f"段 2 起始页: {start_page}")

        f.seek(DATA_REGION_START + start_page * PAGE_SIZE)
        page_data = f.read(PAGE_SIZE)
        
        # 扫描前 100 个 word
        words = struct.unpack(">512I", page_data)
        for i in range(20):
            w = words[i]
            name = decode_base27(w)
            print(f"  [{i}] 0x{w:08X} ({w}) -> {name}")

if __name__ == "__main__":
    analyze()
