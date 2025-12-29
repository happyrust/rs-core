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
        end_page = ptrs[3] # 段 2 结束于段 3 开始

        print(f"段 2: 页 {start_page} 到 {end_page}")

        all_words = []
        for p in range(start_page, end_page):
            f.seek(DATA_REGION_START + p * PAGE_SIZE)
            page_data = f.read(PAGE_SIZE)
            words = struct.unpack(">512I", page_data)
            all_words.extend(words)

        # 假设每 2 个 word 是一个 Noun 记录
        # 第 0 个 word 是 hash，第 1 个 word 是某种 pointer
        for i in range(len(all_words) // 2):
            noun_hash = all_words[i*2]
            noun_ptr = all_words[i*2 + 1]
            if i == 907 or i == 906:
                name = decode_base27(noun_hash)
                print(f"  NounID {i}: Hash=0x{noun_hash:08X} ({noun_hash}) -> {name}, Ptr=0x{noun_ptr:08X}")
            
            # 搜索 PIPE
            name = decode_base27(noun_hash)
            if name == "PIPE":
                print(f"  找到 PIPE: NounID {i}, Hash=0x{noun_hash:08X}, Ptr=0x{noun_ptr:08X}")

if __name__ == "__main__":
    analyze()
