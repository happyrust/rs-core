#!/usr/bin/env python3
"""
直接检查ATGTIX-2坐标指向的PRDISP数据
"""

import struct
from pathlib import Path

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
DATA_REGION_START = 0x1000
MIN_HASH = 0x81BF2
MAX_HASH = 0x171FAD39
BASE27_OFFSET = 0x81BF1

def decode_base27(hash_val: int) -> str:
    if hash_val < MIN_HASH or hash_val > MAX_HASH:
        return ""
    k = hash_val - BASE27_OFFSET
    chars = []
    while k > 0:
        c = k % 27
        chars.append(" " if c == 0 else chr(c + 64))
        k //= 27
    return "".join(chars)

def load_pages(filepath: Path) -> list:
    pages = []
    with filepath.open("rb") as fh:
        fh.seek(DATA_REGION_START)
        while True:
            buf = fh.read(PAGE_SIZE)
            if len(buf) < PAGE_SIZE:
                break
            pages.append(list(struct.unpack(">512I", buf)))
    return pages

def parse_atgtix2(pages: list):
    """解析ATGTIX-2索引"""
    # 简化版：假设起始页已知
    start_page = 1920  # 从之前的分析得知
    records = []
    
    page = start_page
    idx = 0
    
    while True:
        if page >= len(pages):
            break
        if idx >= WORDS_PER_PAGE:
            page += 1
            idx = 0
            continue
            
        word = pages[page][idx]
        idx += 1
        
        if word == 0x00000000:  # PAGE_SWITCH
            page += 1
            idx = 0
            continue
        if word == 0xFFFFFFFF:  # SEGMENT_END
            break
        if word < MIN_HASH or word > MAX_HASH:
            continue
            
        # 读取combined值
        if idx >= WORDS_PER_PAGE:
            break
        combined = pages[page][idx]
        idx += 1
        
        # 计算页面和偏移
        data_page = combined // WORDS_PER_PAGE
        data_offset = combined % WORDS_PER_PAGE
        
        records.append({
            'noun_hash': word,
            'noun_name': decode_base27(word),
            'data_page': data_page,
            'data_offset': data_offset,
            'combined': combined
        })
        
    return records

def read_prdisp_at_location(pages: list, page: int, offset: int):
    """在指定位置读取PRDISP数据"""
    if page >= len(pages) or offset >= WORDS_PER_PAGE:
        return None
        
    # PRDISP格式: [count][attr1][attr2]...[attrN]
    count = pages[page][offset]
    if count == 0 or count > 1000:  # 合理性检查
        return None
        
    attrs = []
    for i in range(count):
        attr_offset = offset + 1 + i
        if attr_offset >= WORDS_PER_PAGE:
            break
        attr_hash = pages[page][attr_offset]
        if MIN_HASH <= attr_hash <= MAX_HASH:
            attrs.append(attr_hash)
    
    return attrs

def main():
    import sys
    if len(sys.argv) < 2:
        print("用法: python debug_prdisp.py <dab_file.dat>")
        return
        
    filepath = Path(sys.argv[1])
    if not filepath.exists():
        print(f"文件不存在: {filepath}")
        return
        
    print(f"分析DAB文件: {filepath}")
    pages = load_pages(filepath)
    print(f"加载页面数: {len(pages)}")
    
    # 解析ATGTIX-2索引
    print("\n解析ATGTIX-2索引...")
    records = parse_atgtix2(pages)
    print(f"找到 {len(records)} 个noun记录")
    
    # 检查前几个noun的PRDISP数据
    print("\n检查PRDISP数据:")
    print("Noun\t\tPage\tOffset\tPRDISP Count\tAttributes")
    print("-" * 70)
    
    found_prdisp = 0
    for i, record in enumerate(records[:10]):  # 只检查前10个
        attrs = read_prdisp_at_location(pages, record['data_page'], record['data_offset'])
        if attrs:
            found_prdisp += 1
            attr_names = [decode_base27(h) for h in attrs[:3]]  # 只显示前3个属性名
            print(f"{record['noun_name']}\t\t{record['data_page']}\t{record['data_offset']}\t{len(attrs)}\t\t{attr_names}")
        else:
            print(f"{record['noun_name']}\t\t{record['data_page']}\t{record['data_offset']}\tNone\t\tNo PRDISP data")
    
    print(f"\n找到有效PRDISP数据的noun: {found_prdisp}/{len(records[:10])}")
    
    # 如果找到PRDISP数据，显示详细属性
    if found_prdisp > 0:
        print("\n详细属性示例:")
        for record in records[:3]:
            attrs = read_prdisp_at_location(pages, record['data_page'], record['data_offset'])
            if attrs:
                print(f"\n{record['noun_name']} (hash: 0x{record['noun_hash']:08X}):")
                for j, attr_hash in enumerate(attrs[:10]):  # 最多显示10个属性
                    attr_name = decode_base27(attr_hash)
                    print(f"  {j+1}. 0x{attr_hash:08X} -> {attr_name}")
                if len(attrs) > 10:
                    print(f"  ... 还有 {len(attrs) - 10} 个属性")

if __name__ == "__main__":
    main()
