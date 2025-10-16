#!/usr/bin/env python3
"""
AVEVA E3D attlib.dat 文件解析器
基于 core.dll 逆向工程分析结果编写

文件格式：
- 偏移 0x000: 文件头（字符串标识）
- 偏移 0x800: 8个数据段指针
- 偏移 0x1000+: 实际数据区
"""

import struct
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict


@dataclass
class AttributeInfo:
    """属性信息"""
    hash: int           # 属性哈希ID
    type_code: int      # 类型代码
    flags: int          # 标志位
    offset: int         # 在Noun实例中的偏移
    name: str = ""      # 属性名称
    att_type: str = ""  # 属性类型字符串


@dataclass
class NounInfo:
    """Noun信息"""
    hash: int           # Noun哈希ID
    name: str           # Noun名称
    attributes: List[AttributeInfo]  # 属性列表


class AttlibParser:
    """attlib.dat 解析器"""

    # 属性类型映射（基于 E3D 定义）
    ATT_TYPE_MAP = {
        1: "ELEMENT",    # 元素引用
        2: "INTEGER",    # 整数
        3: "REAL",       # 实数/浮点
        4: "STRING",     # 字符串
        5: "WORD",       # 单词
        6: "BOOL",       # 布尔
        7: "INTVEC",     # 整数数组
        8: "REALVEC",    # 实数数组
    }

    def __init__(self, filename: str):
        self.filename = filename
        self.data = None
        self.nouns: Dict[str, NounInfo] = {}

    def load_file(self):
        """加载文件到内存"""
        with open(self.filename, 'rb') as f:
            self.data = f.read()
        print(f"✓ 加载文件: {self.filename} ({len(self.data)} 字节)")

    def read_int32(self, offset: int, big_endian: bool = True) -> int:
        """读取32位整数"""
        if offset + 4 > len(self.data):
            return 0
        fmt = '>I' if big_endian else '<I'
        return struct.unpack(fmt, self.data[offset:offset+4])[0]

    def read_string(self, offset: int, max_len: int = 256) -> str:
        """读取以NULL结尾的字符串"""
        if offset >= len(self.data):
            return ""
        end = offset
        while end < len(self.data) and end < offset + max_len and self.data[end] != 0:
            end += 1
        try:
            return self.data[offset:end].decode('ascii', errors='ignore')
        except:
            return ""

    def parse_header(self) -> Tuple[List[int], str]:
        """解析文件头"""
        # 读取偏移 0x800 的8个数据段指针
        segment_offsets = []
        for i in range(8):
            offset = 0x800 + i * 4
            segment_offsets.append(self.read_int32(offset))

        # 读取版本信息（从文件头）
        version = ""
        # 前面的数据是编码的字符串 "Attribute Data File"

        print(f"✓ 数据段指针: {[hex(x) for x in segment_offsets]}")
        return segment_offsets, version

    def parse_attribute_data(self, start_offset: int, count: int) -> List[AttributeInfo]:
        """
        解析属性数据段
        每个属性记录格式 (16字节):
          +0: hash (4字节)
          +4: type_code (4字节)
          +8: flags (4字节)
          +12: offset (4字节)
        """
        attributes = []
        offset = start_offset

        for i in range(count):
            if offset + 16 > len(self.data):
                break

            hash_val = self.read_int32(offset)
            type_code = self.read_int32(offset + 4)
            flags = self.read_int32(offset + 8)
            attr_offset = self.read_int32(offset + 12)

            # 跳过无效记录
            if hash_val == 0:
                offset += 16
                continue

            att_type = self.ATT_TYPE_MAP.get(type_code, f"UNKNOWN_{type_code}")

            attr = AttributeInfo(
                hash=hash_val,
                type_code=type_code,
                flags=flags,
                offset=attr_offset,
                att_type=att_type
            )
            attributes.append(attr)
            offset += 16

        return attributes

    def parse(self):
        """解析整个文件"""
        print("\n开始解析 attlib.dat...")

        # 1. 解析文件头
        segment_offsets, version = self.parse_header()

        # 2. 尝试解析属性数据
        # 基于逆向工程，数据段3通常存储Attribute定义
        attr_segment = segment_offsets[3] if len(segment_offsets) > 3 else 0x1000

        print(f"\n尝试从偏移 0x{attr_segment:x} 解析属性数据...")

        # 预估属性数量（根据文件大小）
        estimated_count = min(10000, (len(self.data) - attr_segment) // 16)
        attributes = self.parse_attribute_data(attr_segment, estimated_count)

        print(f"✓ 解析出 {len(attributes)} 个属性定义")

        # 3. 显示前20个属性样本
        print("\n属性样本（前20个）:")
        for i, attr in enumerate(attributes[:20]):
            print(f"  {i+1:3d}. Hash=0x{attr.hash:08x}, Type={attr.att_type:10s}, Offset={attr.offset:3d}, Flags=0x{attr.flags:x}")

        return attributes

    def export_json(self, attributes: List[AttributeInfo], output_file: str):
        """导出为JSON格式"""
        # 按哈希值分组
        attr_dict = {}
        for attr in attributes:
            key = f"attr_{attr.hash:08x}"
            attr_dict[key] = {
                "hash": attr.hash,
                "hash_hex": f"0x{attr.hash:08x}",
                "type": attr.att_type,
                "type_code": attr.type_code,
                "offset": attr.offset,
                "flags": attr.flags,
                "flags_hex": f"0x{attr.flags:x}"
            }

        output_data = {
            "source_file": self.filename,
            "attribute_count": len(attributes),
            "attributes": attr_dict
        }

        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(output_data, f, indent=2, ensure_ascii=False)

        print(f"\n✓ 已导出到: {output_file}")


def main():
    input_file = "data/attlib-2.10.dat"
    output_file = "data/attlib_parsed.json"

    if len(sys.argv) > 1:
        input_file = sys.argv[1]
    if len(sys.argv) > 2:
        output_file = sys.argv[2]

    parser = AttlibParser(input_file)
    parser.load_file()
    attributes = parser.parse()
    parser.export_json(attributes, output_file)

    print("\n✅ 解析完成！")


if __name__ == "__main__":
    main()
