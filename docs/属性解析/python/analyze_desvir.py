#!/usr/bin/env python3
"""
分析desvir.dat文件的结构和用途
"""

import struct
from pathlib import Path

def analyze_file_header(filepath: Path):
    """分析文件头部信息"""
    print("=== 文件头部分析 ===")
    with filepath.open("rb") as fh:
        # 读取前1KB的头部信息
        header = fh.read(1024)
        
        # 分析可能的字符串
        strings = []
        current = ""
        for byte in header:
            if 32 <= byte <= 126:  # 可打印字符
                current += chr(byte)
            else:
                if len(current) > 3:  # 长度大于3的字符串
                    strings.append(current)
                current = ""
        
        print("发现的字符串:")
        for s in strings[:10]:  # 只显示前10个
            print(f"  {s}")
        
        # 分析文件头部的数值结构
        print("\n头部数值结构 (前64字节):")
        fh.seek(0)
        header_64 = fh.read(64)
        as_dwords = struct.unpack(">16I", header_64)
        for i, val in enumerate(as_dwords):
            print(f"  [{i:2d}] 0x{val:08X} ({val})")

def analyze_field_structure(pages: list, field_hashes: list):
    """分析字段结构模式"""
    print("\n=== 字段结构分析 ===")
    
    # 统计字段类型
    type_counts = {}
    for _, field_type, _ in field_hashes:
        type_counts[field_type] = type_counts.get(field_type, 0) + 1
    
    print("字段类型分布:")
    for field_type, count in sorted(type_counts.items()):
        print(f"  类型 {field_type}: {count} 个字段")
    
    # 分析字段名称模式
    print("\n字段名称模式分析:")
    ref_fields = [name for _, _, name in field_hashes if name.endswith('REF')]
    other_fields = [name for _, _, name in field_hashes if not name.endswith('REF')]
    
    print(f"  REF类型字段: {len(ref_fields)} 个")
    if ref_fields:
        print(f"    示例: {ref_fields[:5]}")
    
    print(f"  其他类型字段: {len(other_fields)} 个")
    if other_fields:
        print(f"    示例: {other_fields[:5]}")

def analyze_page_structure(pages: list):
    """分析页面结构"""
    print("\n=== 页面结构分析 ===")
    
    # 统计每页的数据密度
    page_density = []
    for i, page in enumerate(pages):
        non_zero = sum(1 for word in page if word != 0)
        density = non_zero / len(page)
        page_density.append((i, non_zero, density))
    
    # 显示数据最密集的页面
    page_density.sort(key=lambda x: x[1], reverse=True)
    print("数据最密集的页面 (前10个):")
    for page_idx, non_zero, density in page_density[:10]:
        print(f"  页面 {page_idx}: {non_zero}/512 非零字 ({density:.2%})")
    
    # 查找特殊标记
    page_switches = 0
    segment_ends = 0
    for page in pages:
        page_switches += page.count(0x00000000)
        segment_ends += page.count(0xFFFFFFFF)
    
    print(f"\n特殊标记统计:")
    print(f"  PAGE_SWITCH (0x00000000): {page_switches} 次")
    print(f"  SEGMENT_END (0xFFFFFFFF): {segment_ends} 次")

def main():
    filepath = Path("/Volumes/DPC/work/plant-code/rs-core/data/desvir.dat")
    
    print(f"分析文件: {filepath}")
    print(f"文件大小: {filepath.stat().st_size} 字节")
    
    # 分析文件头部
    analyze_file_header(filepath)
    
    # 加载页面数据
    PAGE_SIZE = 2048
    DATA_REGION_START = 0x1000
    
    pages = []
    with filepath.open("rb") as fh:
        fh.seek(DATA_REGION_START)
        page_count = 0
        while True:
            buf = fh.read(PAGE_SIZE)
            if len(buf) < PAGE_SIZE:
                break
            page = list(struct.unpack(">512I", buf))
            pages.append(page)
            page_count += 1
    
    print(f"\n=== 数据区域分析 ===")
    print(f"数据区域起始: 0x{DATA_REGION_START:08X}")
    print(f"页面数量: {page_count}")
    print(f"数据区域大小: {page_count * PAGE_SIZE} 字节")
    
    # 解析字段（简化版ATGTDF-2解析）
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
    
    field_hashes = []
    for page_idx, page in enumerate(pages):
        for i in range(0, len(page) - 1, 2):  # 假设字段是成对的
            field_hash = page[i]
            field_type = page[i + 1]
            
            if MIN_HASH <= field_hash <= MAX_HASH:
                field_name = decode_base27(field_hash)
                field_hashes.append((field_hash, field_type, field_name))
    
    print(f"找到 {len(field_hashes)} 个可能的字段定义")
    
    # 分析字段结构
    analyze_field_structure(pages, field_hashes)
    
    # 分析页面结构
    analyze_page_structure(pages)
    
    # 结论
    print("\n=== 分析结论 ===")
    print("基于以上分析，desvir.dat可能是:")
    print("1. 项目特定的引用数据库 (大量*REF字段)")
    print("2. 模式定义文件 (包含字段类型信息)")
    print("3. 不是标准的PDMS运行时DAB数据库")
    print("4. 可能用于存储项目间的引用关系和元数据")

if __name__ == "__main__":
    main()
