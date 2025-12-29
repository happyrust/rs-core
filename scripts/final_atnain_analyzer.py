import struct
import json

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
        
        # 加载所有数据 Word
        f.seek(DATA_REGION_START)
        raw_data = f.read()
        all_words = struct.unpack(f">{len(raw_data)//4}I", raw_data)

    # 1. 建立 NounID -> NounName 映射
    # 策略: 搜索 [1, 1, 1, NameHash, count, OffsetInAnotherSegment, 1, AttrHash1, AttrHash2, ...] 模式
    # 虽然 NounID 没直接在这里，但我们可以通过 Segment 4 的启发式找到它
    
    noun_id_to_hash = {}
    # 基于 generate_attr_info_v2.py 的逻辑
    for i in range(len(all_words) - 10):
        # POS(545713) 常作为第一个或核心属性
        if all_words[i] == 545713 and 0 < all_words[i+1] < 2000:
            noun_id = all_words[i+1]
            # 向上找 NounHash。通常 Noun 定义在前面。
            # 这是一个启发式搜索
            for j in range(max(0, i-50), i):
                if all_words[j] == 1 and all_words[j+1] == 1 and all_words[j+2] == 1:
                    noun_hash = all_words[j+3]
                    noun_id_to_hash[noun_id] = noun_hash
                    break

    print(f"建立 NounID 映射: 已找到 {len(noun_id_to_hash)} 个对应关系")
    if 907 in noun_id_to_hash:
        print(f"  Confirm: NounID 907 -> {decode_base27(noun_id_to_hash[907])}")
    else:
        # 如果没找到，尝试全量扫描 noun_id
        print("  未直接找到 907 的映射，尝试其他模式...")

    # 2. 解析 ATNAIN (段 3)
    atnain_start = ptrs[3]
    atnain_end = ptrs[4]
    
    bindings = {}
    for p in range(atnain_start, atnain_end):
        p_words = all_words[p*512 : (p+1)*512]
        for i in range(0, 512-2, 3):
            attr_id = p_words[i]
            noun_id = p_words[i+1]
            offset = p_words[i+2]
            
            if attr_id == 0: continue
            if attr_id == 0xFFFFFFFF: break
            
            if attr_id >= 531442:
                if noun_id not in bindings: bindings[noun_id] = {}
                bindings[noun_id][attr_id] = offset

    # 3. 输出汇总
    report = {}
    for nid, attrs in bindings.items():
        nname = decode_base27(noun_id_to_hash.get(nid, nid))
        attr_report = {}
        for aid, off in attrs.items():
            aname = decode_base27(aid)
            attr_report[aname] = off
        report[nname] = attr_report

    # 打印一些关键 Noun 的偏移
    for target in ["PIPE", "ELBO", "VALV", "FLAN"]:
        if target in report:
            print(f"\n{target} 物理偏移:")
            for aname, off in sorted(report[target].items(), key=lambda x: x[1]):
                print(f"  {aname:<10}: {off}")
        else:
            # 也许是 ID 形式
            pass

    with open("atnain_report.json", "w") as f:
        json.dump(report, f, indent=2)

if __name__ == "__main__":
    analyze()
