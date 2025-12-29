#!/usr/bin/env python3
"""
generate_attr_info.py - 从 attlib.dat 生成 all_attr_info.json
基于最新的 Segment 4 分析结论。
"""
import struct
import json
import sys
import os
from typing import Dict, List, Optional, Any

# Constants
ATTLIB_PAGE_SIZE = 2048
ATTLIB_WORDS_PER_PAGE = 512
ATTLIB_DATA_REGION_START = 0x1000
ATTLIB_SEGMENT_POINTERS_OFFSET = 0x0800
ATTLIB_MIN_HASH = 100000
ATTLIB_MAX_HASH = 999999999

# Known Data Type Sizes (in Words)
TYPE_SIZES = {
    "POSITION": 6,     # 3 Double = 24 bytes
    "ORIENTATION": 6,  # 3 Double = 24 bytes
    "DOUBLE": 2,       # 1 Double = 8 bytes
    "INTEGER": 1,      # 1 Int = 4 bytes
    "BOOL": 1,         # 1 Int
    "STRING": 0,       # Variable, typically offset 0 or special handling
    "WORD": 1,
    "ELEMENT": 2       # 2 Ints (Ref0, Ref1)
}

class AttlibParser:
    def __init__(self, attlib_path: str, ref_json_path: Optional[str] = None):
        self.attlib_path = attlib_path
        self.ref_json_path = ref_json_path
        self.words: List[int] = []
        self.hash_to_name: Dict[int, str] = {}
        self.hash_to_type: Dict[int, str] = {}
        self.noun_to_attr_list: Dict[int, List[int]] = {}
        self.final_map: Dict[str, Dict[str, dict]] = {}
        
    def parse(self):
        if self.ref_json_path:
            self._load_reference_names()
        
        with open(self.attlib_path, 'rb') as f:
            f.seek(ATTLIB_DATA_REGION_START)
            data = f.read()
            self.words = list(struct.unpack(f'>{len(data)//4}I', data))

        self._load_attr_types()
        self._scan_segment_4()
        self._calculate_offsets()
        
    def _load_reference_names(self):
        print(f"Loading names from {self.ref_json_path}...")
        with open(self.ref_json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        for noun_id, attrs in data.get('noun_attr_info_map', {}).items():
            for h_str, info in attrs.items():
                self.hash_to_name[int(h_str)] = info.get('name', f"unk_{h_str}")
        
    def _load_attr_types(self):
        print("Scanning attribute types...")
        for i in range(len(self.words) - 2):
            h = self.words[i]
            if ATTLIB_MIN_HASH <= h <= ATTLIB_MAX_HASH:
                dtype = self.words[i+1]
                if 1 <= dtype <= 20: # Wider range
                    from_map = {1:"INTEGER", 2:"DOUBLE", 3:"BOOL", 4:"STRING", 5:"WORD", 6:"ELEMENT", 7:"POSITION", 8:"ORIENTATION"}.get(dtype)
                    if from_map:
                        self.hash_to_type[h] = from_map

    def _scan_segment_4(self):
        print("Scanning Segment 4 for Noun Attribute Lists...")
        # 寻找模式: AttrID_of_Noun, NounID, Pointer, Attr_sequence...
        # 例如: [545713, 11, 817539, 860074, 750558, 10206636, 545713, 538503, ...]
        for i in range(len(self.words) - 20):
            # 启发式: 一个合法的 NounID 紧跟在一个指向字符串的 Pointer 之前
            # 并且前面是一个已知的 AttrID (POS=545713)
            if self.words[i] == 545713 and 0 < self.words[i+1] < 1000 and self.words[i+2] > 500000:
                noun_id = self.words[i+1]
                # 寻找后续的属性序列 (直到遇到非 AttrID)
                attrs = []
                j = i + 3
                while j < len(self.words) and (ATTLIB_MIN_HASH <= self.words[j] <= ATTLIB_MAX_HASH or self.words[j] < 500):
                    if self.words[j] > 100000:
                        attrs.append(self.words[j])
                    j += 1
                if attrs:
                    self.noun_to_attr_list[noun_id] = attrs
        print(f"Found {len(self.noun_to_attr_list)} unique Noun attribute lists.")

    def _calculate_offsets(self):
        print("Calculating physical offsets...")
        # 实现一个简化的偏移量计算逻辑
        for noun_id, attrs in self.noun_to_attr_list.items():
            noun_key = str(noun_id)
            self.final_map[noun_key] = {}
            
            current_offset = 0
            for attr_hash in attrs:
                name = self.hash_to_name.get(attr_hash, f"unk_{attr_hash}")
                atype = self.hash_to_type.get(attr_hash, "UNKNOWN")
                
                # 特殊处理已知偏移 (基于之前对 Noun 11 的 POS=11 的观察)
                # 这部分需要更复杂的启发式或硬编码一些基准
                
                self.final_map[noun_key][str(attr_hash)] = {
                    "name": name,
                    "hash": attr_hash,
                    "offset": current_offset,
                    "att_type": atype
                }
                
                # 累加偏移
                size = TYPE_SIZES.get(atype, 1)
                current_offset += size

    def save(self, path):
        with open(path, 'w', encoding='utf-8') as f:
            json.dump({"noun_attr_info_map": self.final_map}, f, indent=2)
        print(f"Saved to {path}")

if __name__ == "__main__":
    p = AttlibParser(sys.argv[1], sys.argv[3] if len(sys.argv) > 3 else None)
    p.parse()
    p.save(sys.argv[2])
