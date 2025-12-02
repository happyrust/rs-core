#!/usr/bin/env python3
"""
调试ATGTDF-2段内容，查找所有可用的字段哈希
"""

import struct
from pathlib import Path

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
DATA_REGION_START = 0x1000
PAGE_SWITCH = 0x00000000
SEGMENT_END = 0xFFFFFFFF
MIN_HASH = 0x81BF2
MAX_HASH = 0x171FAD39

def decode_base27(hash_val: int) -> str:
    if hash_val < MIN_HASH or hash_val > MAX_HASH:
        return ""
    k = hash_val - 0x81BF1
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

def parse_atgtdf2(pages: list, start_page: int = 0):
    """解析ATGTDF-2段，返回字段哈希列表"""
    page = start_page
    idx = 0
    field_hashes = []
    
    while True:
        if page >= len(pages):
            break
        if idx >= WORDS_PER_PAGE:
            page += 1
            idx = 0
            continue
            
        word = pages[page][idx]
        idx += 1
        
        if word == PAGE_SWITCH:
            page += 1
            idx = 0
            continue
        if word == SEGMENT_END:
            break
        if word < MIN_HASH or word > MAX_HASH:
            continue
            
        # ATGTDF-2格式: [field_hash][field_type]...
        field_hash = word
        field_type = pages[page][idx] if idx < WORDS_PER_PAGE else 0
        idx += 1
        
        field_hashes.append((field_hash, field_type, decode_base27(field_hash)))
        
    return field_hashes

def find_atgtdf2_start(pages: list):
    """寻找ATGTDF-2段的起始页"""
    best_page = -1
    max_count = 0
    
    for page_idx in range(len(pages)):
        # 快速筛选：页内至少有一个有效哈希
        if not any(MIN_HASH <= w <= MAX_HASH for w in pages[page_idx]):
            continue
            
        hashes = parse_atgtdf2(pages, page_idx)
        if len(hashes) > max_count:
            max_count = len(hashes)
            best_page = page_idx
            
    return best_page, max_count

def main():
    import sys
    if len(sys.argv) < 2:
        print("用法: python debug_atgtdf2.py <file.dat>")
        return
        
    filepath = Path(sys.argv[1])
    if not filepath.exists():
        print(f"文件不存在: {filepath}")
        return
        
    print(f"分析文件: {filepath}")
    pages = load_pages(filepath)
    print(f"加载页面数: {len(pages)}")
    
    # 查找ATGTDF-2起始页
    start_page, count = find_atgtdf2_start(pages)
    print(f"ATGTDF-2 起始页: {start_page}, 字段数: {count}")
    
    if start_page >= 0:
        field_hashes = parse_atgtdf2(pages, start_page)
        print(f"\n找到 {len(field_hashes)} 个字段:")
        print("Hash (hex)\tHash (dec)\tType\tName")
        print("-" * 60)
        
        for field_hash, field_type, name in field_hashes[:20]:  # 只显示前20个
            print(f"0x{field_hash:08X}\t{field_hash}\t{field_type}\t{name}")
        
        if len(field_hashes) > 20:
            print(f"... 还有 {len(field_hashes) - 20} 个字段")
            
        # 检查PRDISP是否存在
        prdisp_hash = 0x0E5416D9
        prdisp_found = any(fh == prdisp_hash for fh, _, _ in field_hashes)
        print(f"\nPRDISP (0x{prdisp_hash:08X}) 存在: {prdisp_found}")
        
        # 查找可能的PRDISP候选
        prdisp_candidates = [fh for fh, _, name in field_hashes if 'PRD' in name or 'DISP' in name]
        if prdisp_candidates:
            print(f"可能的PRDISP候选: {[f'0x{h:08X}' for h in prdisp_candidates]}")

if __name__ == "__main__":
    main()
