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
        f.seek(SEGMENT_POINTERS_OFFSET)
        ptrs = struct.unpack(">8I", f.read(32))
        start_page = ptrs[2]
        end_page = ptrs[3]

        all_words = []
        for p in range(start_page, end_page):
            f.seek(DATA_REGION_START + p * PAGE_SIZE)
            page_data = f.read(PAGE_SIZE)
            words = struct.unpack(">512I", page_data)
            all_words.extend(words)

        # 尝试不同的记录步长
        results = []
        # 如果是 4 个 word 一组？
        for step in [2, 3, 4, 6]:
            print(f"\n尝试步长 {step}:")
            for i in range(min(1000, len(all_words) // step)):
                noun_hash = all_words[i*step]
                name = decode_base27(noun_hash)
                if name == "PIPE":
                    print(f"  找到 PIPE! Step={step}, Index={i}, Hash=0x{noun_hash:08X}")
                if i == 907:
                    print(f"  ID 907: Step={step}, Hash=0x{noun_hash:08X}, Name='{name}'")

if __name__ == "__main__":
    analyze()
