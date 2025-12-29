#!/usr/bin/env python3
"""
generate_attr_info.py - 从 attlib.dat 生成 all_attr_info.json

基于分析结论：
1. Noun 定义格式: Description (ASCII) + [1,1,1] + TypeHash + [count, index, flag] + Attr Hashes
2. attlib.dat Segment 3 (ATNAIN) 包含 [AttrID, NounID, Offset] 绑定
3. 属性名称从现有 all_attr_info.json 加载（反向映射）
"""
import struct
import json
import sys
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass

# Constants
ATTLIB_PAGE_SIZE = 2048
ATTLIB_WORDS_PER_PAGE = 512
ATTLIB_DATA_REGION_START = 0x1000
ATTLIB_SEGMENT_POINTERS_OFFSET = 0x0800
ATTLIB_PAGE_SWITCH_MARK = 0xFFFFFFFF
ATTLIB_SEGMENT_END_MARK = 0xFFFFFFFE
ATTLIB_MIN_HASH = 100000
ATTLIB_MAX_HASH = 999999999

# Data Types Mapping
DATA_TYPE_MAP = {
    1: "INTEGER",
    2: "DOUBLE",
    3: "BOOL",
    4: "STRING",
    5: "WORD",
    6: "ELEMENT",
    7: "POSITION",
    8: "ORIENTATION",
    9: "DIRECTION",
    10: "INTVEC",
}

DEFAULT_VALUES = {
    "INTEGER": {"IntegerType": 0},
    "DOUBLE": {"DoubleType": 0.0},
    "BOOL": {"BoolType": False},
    "STRING": {"StringType": ""},
    "WORD": {"WordType": "unset"},
    "ELEMENT": {"ElementType": ""},
    "POSITION": {"Vec3Type": [0.0, 0.0, 0.0]},
    "ORIENTATION": {"Vec3Type": [0.0, 0.0, 0.0]},
    "DIRECTION": {"Vec3Type": [0.0, 0.0, 0.0]},
    "INTVEC": {"IntArrayType": []},
}

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

@dataclass
class NounDefinition:
    type_hash: int
    attr_hashes: List[int]
    description: str = ""

class AttlibParser:
    def __init__(self, attlib_path: str, ref_json_path: Optional[str] = None):
        self.attlib_path = attlib_path
        self.ref_json_path = ref_json_path
        self.file = None
        self.words: List[int] = []
        self.segment_pointers: List[int] = []
        self.hash_to_name: Dict[int, str] = {}
        self.hash_to_type: Dict[int, str] = {}
        self.noun_definitions: Dict[int, NounDefinition] = {}  # TypeHash -> NounDef
        self.noun_bindings: Dict[str, Dict[str, dict]] = {}  # TypeHash -> {AttrHash -> info}
        
    def parse(self):
        if self.ref_json_path:
            self._load_reference_names()
        
        with open(self.attlib_path, 'rb') as f:
            f.seek(ATTLIB_SEGMENT_POINTERS_OFFSET)
            for _ in range(8):
                data = f.read(4)
                if len(data) == 4:
                    self.segment_pointers.append(struct.unpack('>I', data)[0])
            
            f.seek(ATTLIB_DATA_REGION_START)
            data = f.read()
            self.words = list(struct.unpack(f'>{len(data)//4}I', data))
        
        print(f"Segment Pointers: {self.segment_pointers}")
        print(f"Total words: {len(self.words)}")
        
        self._load_attr_types()
        self._scan_noun_definitions()
        self._build_noun_attr_map()
    
    def _load_reference_names(self):
        """从现有 all_attr_info.json 加载 Hash -> Name 映射"""
        print(f"Loading reference names from {self.ref_json_path}...")
        with open(self.ref_json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        for noun_id, attrs in data.get('noun_attr_info_map', {}).items():
            for attr_hash, info in attrs.items():
                if 'name' in info and 'hash' in info:
                    self.hash_to_name[info['hash']] = info['name']
        
        print(f"Loaded {len(self.hash_to_name)} name mappings.")
    
    def _scan_noun_definitions(self):
        """扫描 Noun 定义: [1,1,1] TypeHash [count, startIdx, flag] AttrHashes..."""
        print("Scanning Noun definitions...")
        
        # 首先加载 ATNAIN 以获取所有可能的 NounID 和 Hash 关联
        self._load_atnain()
        
        # 记录已扫描出的 Hash
        i = 0
        while i < len(self.words) - 20:
            if self.words[i] == 1 and self.words[i+1] == 1 and self.words[i+2] == 1:
                # Check for 5-ones (PIPE style): 1,1,1, 1,1, Hash
                is_five_ones = False
                if self.words[i+3] == 1 and self.words[i+4] == 1:
                    is_five_ones = True
                
                type_hash = 0
                count = 0
                attr_start_idx = 0
                
                if is_five_ones:
                    # Pattern: 1,1,1,1,1, Type(i+5), ?(i+6), Count(i+7), ?(i+8), Attrs(i+9...)
                    type_hash = self.words[i+5]
                    count = self.words[i+7]
                    attr_start_idx = i + 9
                else:
                    # Pattern: 1,1,1, Type(i+3), Count(i+4), ?, ?, Attrs(i+7...)
                    type_hash = self.words[i+3]
                    count = self.words[i+4]
                    attr_start_idx = i + 7 # Based on prev code
                
                if ATTLIB_MIN_HASH <= type_hash <= ATTLIB_MAX_HASH and 0 < count < 5000:
                    if type_hash not in self.noun_definitions:
                        attr_hashes = []
                        curr = attr_start_idx
                        # Collect next 'count' VALID hashes?
                        # Or assume contiguous block?
                        # PIPE has clean list.
                        added = 0
                        # Scan a bit more than count to skip small ints?
                        for k in range(min(count * 2, 5000)):
                            if curr >= len(self.words): break
                            if added >= count: break
                            
                            h = self.words[curr]
                            if ATTLIB_MIN_HASH <= h <= ATTLIB_MAX_HASH:
                                attr_hashes.append(h)
                                added += 1
                            curr += 1
                        
                        self.noun_definitions[type_hash] = NounDefinition(type_hash=type_hash, attr_hashes=attr_hashes)
                        # Advance iterator? 
                        # To be safe, just increment by 1 is fine, but slow.
                        # Let's skip passed count.
                        # i = curr
                        # But be careful not to skip next start adjacent?
                        # Just let the loop proceed.
            i += 1

        # 补全那些出现在 ATNAIN 但没在 [1,1,1] 中被识别出的 Noun
        for noun_id, noun_hash in self.noun_id_to_hash.items():
            if noun_hash not in self.noun_definitions:
                # 这些 Noun 的属性列表可能需要从 ATNAIN 的绑定中反推
                if noun_hash in self.atnain_mappings:
                    attr_hashes = list(self.atnain_mappings[noun_hash].keys())
                    self.noun_definitions[noun_hash] = NounDefinition(type_hash=noun_hash, attr_hashes=attr_hashes)
        
        print(f"Total Noun definitions: {len(self.noun_definitions)}")

    def _load_atnain(self):
        """加载 ATNAIN (Noun-属性物理偏移绑定)"""
        print("Loading ATNAIN bindings (Segment 3)...")
        if len(self.segment_pointers) < 4:
            print("Warning: Segment pointers insufficient for ATNAIN")
            return

        atnain_start_page = self.segment_pointers[3]
        if atnain_start_page == 0:
            print("Warning: ATNAIN segment pointer is 0")
            return

        # 获取数据页中的起始偏移
        start_word_idx = (atnain_start_page * ATTLIB_WORDS_PER_PAGE)
        
        # 建立 NounID -> NounHash 的映射
        self.noun_id_to_hash = {}
        
        # 1. 扫描段 2 (Noun 索引表): 建立 基于位置的 NounID 映射
        # 基于分析，NounID 4 -> NAME 在页 1463 (段 4?) 的观察
        # 段 2 (页 1415-1433) 包含 Noun 定义索引。
        if len(self.segment_pointers) >= 3:
            s2_start = self.segment_pointers[2] * ATTLIB_WORDS_PER_PAGE
            s2_end = self.segment_pointers[3] * ATTLIB_WORDS_PER_PAGE
            
            # 记录所有在段 2 中发现的 NounHash
            noun_hashes_in_order = []
            for i in range(s2_start, s2_end - 1, 2):
                h = self.words[i]
                if ATTLIB_MIN_HASH <= h <= ATTLIB_MAX_HASH:
                    noun_hashes_in_order.append(h)
            
            # 通常 NounID 是 1-indexed 或者 0-indexed 的序号
            for idx, h in enumerate(noun_hashes_in_order):
                # 尝试 0-indexed 和 1-indexed (根据 PIPE=907 的采样，这里可能是 1-indexed)
                self.noun_id_to_hash[idx] = h      # 0-indexed
                self.noun_id_to_hash[idx + 1] = h  # 1-indexed
        
        print(f"Established {len(self.noun_id_to_hash)} potential NounID mappings from Segment 2")

        # 2. 启发式补充：从数据块中直接提取确定的绑定 (如 POS 所在的元数据块)
        for i in range(len(self.words) - 20):
            if self.words[i] == 545713 and 0 < self.words[i+1] < 10000:
                noun_id = self.words[i+1]
                for j in range(max(0, i-50), i):
                    if j+3 < len(self.words) and self.words[j] == 1 and self.words[j+1] == 1 and self.words[j+2] == 1:
                        self.noun_id_to_hash[noun_id] = self.words[j+3]
                        break

        # 解析 ATNAIN 三元组 [AttrHash, NounID, Offset]
        self.atnain_mappings = {} # NounHash -> {AttrHash -> Offset}
        
        curr_idx = start_word_idx
        while curr_idx + 2 < len(self.words):
            attr_hash = self.words[curr_idx]
            noun_id = self.words[curr_idx + 1]
            offset = self.words[curr_idx + 2]
            
            if attr_hash == 0xFFFFFFFF or (attr_hash == 0 and noun_id == 0):
                break
                
            if ATTLIB_MIN_HASH <= attr_hash <= ATTLIB_MAX_HASH:
                noun_hash = self.noun_id_to_hash.get(noun_id)
                if noun_hash:
                    if noun_hash not in self.atnain_mappings:
                        self.atnain_mappings[noun_hash] = {}
                    self.atnain_mappings[noun_hash][attr_hash] = offset
            
            curr_idx += 3
        
        print(f"Loaded physical offsets for {len(self.atnain_mappings)} unique Noun hashes from ATNAIN")
    
    def _load_attr_types(self):
        """从 Segment 0 扫描属性类型定义"""
        print("Loading Attribute Types...")
        
        for i in range(len(self.words) - 2):
            h = self.words[i]
            if ATTLIB_MIN_HASH <= h <= ATTLIB_MAX_HASH:
                dtype = self.words[i + 1]
                if 1 <= dtype <= 10:
                    if h not in self.hash_to_type:
                        self.hash_to_type[h] = DATA_TYPE_MAP.get(dtype, f"UNKNOWN_{dtype}")
        
        print(f"Loaded {len(self.hash_to_type)} type definitions.")
    
    def _build_noun_attr_map(self):
        """基于 Noun 定义构建最终映射"""
        print("Building Noun-Attribute map...")
        self._load_atnain()
        
        for type_hash, noun_def in self.noun_definitions.items():
            type_hash_str = str(type_hash)
            self.noun_bindings[type_hash_str] = {}
            
            # 获取该 Noun 的物理偏移表
            offset_map = self.atnain_mappings.get(type_hash, {})
            
            for attr_hash in noun_def.attr_hashes:
                attr_hash_str = str(attr_hash)
                name = self.hash_to_name.get(attr_hash)
                
                # 如果名称未找到或为 unk_，尝试 Base27 解码
                if not name or name.startswith("unk_"):
                    # 尝试解码
                    decoded = decode_base27(attr_hash)
                    if decoded and not decoded.startswith("unk_"):
                        name = decoded
                    elif not name:
                         name = f"unk_{attr_hash}"

                att_type = self.hash_to_type.get(attr_hash, "UNKNOWN")
                default_val = DEFAULT_VALUES.get(att_type, {})
                
                # 从 ATNAIN 中获取物理偏移，如果没有则默认为 0
                offset = offset_map.get(attr_hash, 0)
                
                self.noun_bindings[type_hash_str][attr_hash_str] = {
                    "name": name,
                    "hash": attr_hash,
                    "offset": offset,
                    "default_val": default_val,
                    "att_type": att_type
                }
        
        # Post-process: Alias ELBO (828473) if missing
        elbo_hash = 828473
        pipe_hash = 641779
        bend_hash = 620516
        
        elbo_str = str(elbo_hash)
        pipe_str = str(pipe_hash)
        bend_str = str(bend_hash)
        
        if elbo_str not in self.noun_bindings:
            print("ELBO not found, aliasing to PIPE + BEND attributes...")
            combined_attrs = {}
            if pipe_str in self.noun_bindings:
                combined_attrs.update(self.noun_bindings[pipe_str])
            if bend_str in self.noun_bindings:
                combined_attrs.update(self.noun_bindings[bend_str])
            
            if combined_attrs:
                self.noun_bindings[elbo_str] = combined_attrs
                print(f"Created ELBO with {len(combined_attrs)} attributes.")

        print(f"Built map for {len(self.noun_bindings)} nouns")
    
    def save_json(self, output_path: str):
        data = {"noun_attr_info_map": self.noun_bindings}
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        print(f"Saved to {output_path}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 generate_attr_info.py <attlib.dat> <output.json> [ref.json]")
        sys.exit(1)
    
    parser = AttlibParser(sys.argv[1], sys.argv[3] if len(sys.argv) > 3 else None)
    parser.parse()
    parser.save_json(sys.argv[2])
