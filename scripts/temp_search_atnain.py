import struct

ATTLIB_PATH = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"
PAGE_SIZE = 2048
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x0800

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

def analyze():
    target_attr_hash = 639374 # NAME
    with open(ATTLIB_PATH, "rb") as f:
        f.seek(SEGMENT_POINTERS_OFFSET)
        ptrs = struct.unpack(">8I", f.read(32))
        start_page = ptrs[3]
        end_page = ptrs[4]

        print(f"ATNAIN 段: 页 {start_page} 到 {end_page}")

        for p in range(start_page, end_page):
            f.seek(DATA_REGION_START + p * PAGE_SIZE)
            page_data = f.read(PAGE_SIZE)
            # 每 12 字节一个三元组
            for i in range(PAGE_SIZE // 12):
                chunk = page_data[i*12 : (i+1)*12]
                if len(chunk) < 12: break
                attr_id, noun_id, offset = struct.unpack(">3I", chunk)
                
                if attr_id == target_attr_hash:
                    # 解码 noun_id
                    # 假设 noun_id 也是一种 hash? 或者我们需要找到 907
                    print(f"  找到 NAME: NounID={noun_id}, Offset={offset} (页 {p}, 偏移 {i*12})")
                
                if noun_id == 907:
                    attr_name = decode_base27(attr_id)
                    print(f"  NounID 907 绑定: Attr={attr_name}({attr_id}), Offset={offset}")

if __name__ == "__main__":
    analyze()
