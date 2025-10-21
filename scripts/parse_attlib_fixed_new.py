#!/usr/bin/env python3
"""
attlib.dat 解析器 - Python 版本
用于快速测试和调试属性库文件格式
"""

import struct
import json
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict

# 常量定义
PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
MIN_HASH = 531442
MAX_HASH = 387951929
PAGE_SWITCH_MARK = 0x00000000
SEGMENT_END_MARK = 0xFFFFFFFF
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x0800

@dataclass
class AttlibAttrIndex:
    attr_hash: int
    combined: int
    
    def record_num(self) -> int:
        return self.combined // 512
    
    def slot_offset(self) -> int:
        return self.combined % 512

@dataclass
class AttlibAttrDefinition:
    attr_hash: int
    data_type: int
    default_flag: int
    default_value: any

class AttlibParser:
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.file = open(file_path, 'rb')
        self.attr_index: Dict[int, AttlibAttrIndex] = {}
        self.attr_definitions: Dict[int, AttlibAttrDefinition] = {}
        self.segment_pointers = self._read_segment_pointers()
        self.page_cache: Dict[int, List[int]] = {}
    
    def _read_segment_pointers(self) -> List[int]:
        """读取段指针表"""
        self.file.seek(SEGMENT_POINTERS_OFFSET)
        pointers = []
        for i in range(8):
            data = self.file.read(4)
            ptr = struct.unpack('>I', data)[0]  # 大端序
            pointers.append(ptr)
        
        print("段指针表:")
        for i, ptr in enumerate(pointers):
            print(f"  段 {i}: 0x{ptr:08X} (页号: {ptr})")
        
        return pointers
    
    def read_page(self, page_num: int) -> List[int]:
        """读取指定页号的页面"""
        if page_num in self.page_cache:
            return self.page_cache[page_num]
        
        file_offset = DATA_REGION_START + page_num * PAGE_SIZE
        self.file.seek(file_offset)
        page_data = self.file.read(PAGE_SIZE)
        
        words = []
        for i in range(WORDS_PER_PAGE):
            word_bytes = page_data[i*4:(i+1)*4]
            word = struct.unpack('>I', word_bytes)[0]  # 大端序
            words.append(word)
        
        self.page_cache[page_num] = words
        return words
    
    def load_atgtdf(self):
        """加载 ATGTDF 段（属性定义表）"""
        print("\n加载 ATGTDF 段（属性定义）")
        # 根据 IDA Pro 分析，段指针 [0] 应该指向 ATGTDF
        start_page = self.segment_pointers[0]
        print(f"  使用段指针 [0] = {start_page} 作为起点")

        page_num = start_page
        word_idx = 0
        record_count = 0

        while True:
            words = self.read_page(page_num)

            while word_idx < WORDS_PER_PAGE:
                word = words[word_idx]
                word_idx += 1

                if word == PAGE_SWITCH_MARK:
                    page_num += 1
                    word_idx = 0
                    break

                if word == SEGMENT_END_MARK:
                    print(f"  ATGTDF 加载完成，共 {record_count} 条记录")
                    return
                
                # 检查是否为有效哈希值
                if word < MIN_HASH or word > MAX_HASH:
                    continue
                
                attr_hash = word
                
                # 读取 data_type
                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)
                
                data_type = words[word_idx]
                word_idx += 1
                
                # 读取 default_flag
                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)
                
                default_flag = words[word_idx]
                word_idx += 1
                
                # 读取默认值
                default_value = self._read_default_value(words, word_idx, page_num, data_type, default_flag)
                
                self.attr_definitions[attr_hash] = AttlibAttrDefinition(
                    attr_hash=attr_hash,
                    data_type=data_type,
                    default_flag=default_flag,
                    default_value=default_value
                )
                
                if record_count < 5:
                    print(f"    [{record_count}] hash=0x{attr_hash:08X}, type={data_type}, flag={default_flag}")
                
                record_count += 1
    
    def _read_default_value(self, words, word_idx, page_num, data_type, default_flag):
        """读取默认值"""
        if default_flag == 1:
            return None
        
        if default_flag != 2:
            return None
        
        if data_type == 4:  # TEXT
            if word_idx >= WORDS_PER_PAGE:
                page_num += 1
                word_idx = 0
                words = self.read_page(page_num)
            
            length = words[word_idx]
            text_data = []
            word_idx += 1
            
            for _ in range(length):
                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)
                
                text_data.append(words[word_idx])
                word_idx += 1
            
            return {"type": "TEXT", "data": text_data}
        else:
            # 标量类型
            if word_idx >= WORDS_PER_PAGE:
                page_num += 1
                word_idx = 0
                words = self.read_page(page_num)
            
            scalar = words[word_idx]
            return {"type": "SCALAR", "data": scalar}
    
    def load_atgtix(self):
        """加载 ATGTIX 段（属性索引表）"""
        print("\n加载 ATGTIX 段（属性索引）")
        print("  从页 0 开始扫描")
        
        page_num = 0
        word_idx = 0
        record_count = 0
        
        while True:
            words = self.read_page(page_num)
            
            while word_idx < WORDS_PER_PAGE:
                word = words[word_idx]
                word_idx += 1
                
                if word == PAGE_SWITCH_MARK:
                    page_num += 1
                    word_idx = 0
                    break
                
                if word == SEGMENT_END_MARK:
                    print(f"  ATGTIX 加载完成，共 {record_count} 条记录")
                    return
                
                if word < MIN_HASH or word > MAX_HASH:
                    continue
                
                attr_hash = word
                
                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)
                
                combined = words[word_idx]
                word_idx += 1
                
                self.attr_index[attr_hash] = AttlibAttrIndex(
                    attr_hash=attr_hash,
                    combined=combined
                )
                
                if record_count < 5:
                    print(f"    [{record_count}] hash=0x{attr_hash:08X}, combined=0x{combined:08X}")
                
                record_count += 1
    
    def load_all(self):
        """加载所有段"""
        self.load_atgtdf()
        self.load_atgtix()
    
    def get_attribute(self, hash_val: int) -> Optional[AttlibAttrDefinition]:
        """获取属性定义"""
        return self.attr_definitions.get(hash_val)
    
    def close(self):
        """关闭文件"""
        self.file.close()

def data_type_to_string(data_type: int) -> str:
    """将数据类型代码转换为字符串"""
    types = {
        1: "LOG",
        2: "REAL",
        3: "INT",
        4: "TEXT"
    }
    return types.get(data_type, f"UNKNOWN({data_type})")

if __name__ == "__main__":
    parser = AttlibParser("data/attlib.dat")
    parser.load_all()

    print("\n=== 所有找到的属性 ===")
    print(f"ATGTIX 索引: {len(parser.attr_index)} 条")
    print(f"ATGTDF 定义: {len(parser.attr_definitions)} 条")

    if parser.attr_definitions:
        print("\nATGTDF 中的属性:")
        for i, (hash_val, attr) in enumerate(list(parser.attr_definitions.items())[:10]):
            print(f"  [{i}] hash=0x{hash_val:08X}, type={data_type_to_string(attr.data_type)}, flag={attr.default_flag}")

    if parser.attr_index:
        print("\nATGTIX 中的所有属性:")
        for i, (hash_val, idx) in enumerate(parser.attr_index.items()):
            print(f"  [{i}] hash=0x{hash_val:08X} ({hash_val}), combined=0x{idx.combined:08X}")

    # 测试 ELBO 属性
    elbo_attrs = {
        "POS": 545713,
        "ORI": 538503,
        "NAME": 639374,
        "TYPE": 642215,
    }

    print("\n=== ELBO 属性查询 ===")
    for name, hash_val in elbo_attrs.items():
        attr = parser.get_attribute(hash_val)
        if attr:
            print(f"✓ {name} (hash: {hash_val})")
            print(f"  data_type: {data_type_to_string(attr.data_type)}")
            print(f"  default_flag: {attr.default_flag}")
            print(f"  default_value: {attr.default_value}")
        else:
            print(f"✗ {name} (hash: {hash_val}) - NOT FOUND")

    parser.close()

