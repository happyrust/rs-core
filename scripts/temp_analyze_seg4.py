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
    with open(ATTLIB_PATH, "rb") as f:
        f.seek(SEGMENT_POINTERS_OFFSET)
        ptrs = struct.unpack(">8I", f.read(32))
        start_page = ptrs[4] # Segment 4
        end_page = ptrs[5]

        print(f"Segment 4: 页 {start_page} 到 {end_page}")

        f.seek(DATA_REGION_START + start_page * PAGE_SIZE)
        data = f.read((end_page - start_page) * PAGE_SIZE)
        words = struct.unpack(f">{len(data)//4}I", data)

        for i in range(len(words) - 10):
            # 寻找 POS(545713) 特征
            if words[i] == 545713 and 0 < words[i+1] < 2000:
                noun_id = words[i+1]
                # 看看 noun_id 前面 5 个 word，有没有看起来像 NounHash 的？
                context = words[max(0, i-5) : i+5]
                context_str = " ".join([f"0x{w:08X}" for w in context])
                print(f"NounID {noun_id} context: {context_str}")
                
                # 解码 context 中的 hash
                for w in context:
                    if 531442 < w < 20000000:
                        name = decode_base27(w)
                        if len(name) >= 3:
                            print(f"  潜在名称: {name} (0x{w:08X})")

if __name__ == "__main__":
    analyze()
