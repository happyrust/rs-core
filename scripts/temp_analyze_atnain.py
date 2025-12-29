import struct

ATTLIB_PATH = "/Volumes/DPC/work/plant-code/aios-parse-pdms-fork/data/attlib.dat"
PAGE_SIZE = 2048
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x0800

def analyze():
    with open(ATTLIB_PATH, "rb") as f:
        # 读取段指针
        f.seek(SEGMENT_POINTERS_OFFSET)
        ptrs_raw = f.read(32)
        ptrs = struct.unpack(">8I", ptrs_raw)
        print("段指针表:")
        for i, p in enumerate(ptrs):
            print(f"  段 {i}: {p}")

        target_page = 1433
        print(f"\n分析页 {target_page}:")
        offset = DATA_REGION_START + target_page * PAGE_SIZE
        f.seek(offset)
        page_data = f.read(PAGE_SIZE)
        
        # 每 12 字节一个三元组 (3 * u32)
        count = PAGE_SIZE // 12
        for i in range(10): # 只看前 10 个
            chunk = page_data[i*12 : (i+1)*12]
            attr_id, noun_id, word_offset = struct.unpack(">3I", chunk)
            print(f"  [{i}] AttrID={attr_id:<8} NounID={noun_id:<5} Offset={word_offset}")

if __name__ == "__main__":
    analyze()
