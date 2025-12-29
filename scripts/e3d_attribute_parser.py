#!/usr/bin/env python3
"""
E3D 属性解析器 - 完整实现

基于对 core.dll 中 db4_get_att_dets 的逆向分析实现。

核心架构:
1. AttlibCache - 模拟 dword_115994E4 (shortened att types) 缓存
2. AttlibDataSource - 模拟 dword_11E1E860 (Fortran 全局表) 数据源
3. E3DAttributeParser - 主解析器，整合元数据和数据库读取

数据流:
    attlib.dat → AttlibDataSource → AttlibCache → E3DAttributeParser

使用方法:
    python e3d_attribute_parser.py <attlib.dat> <database.db> [element_refno]
"""

import struct
import os
import sys
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass, field
from enum import IntEnum


# ============================================================================
# 常量定义 (基于逆向分析)
# ============================================================================

# attlib.dat 常量
ATTLIB_PAGE_SIZE = 2048
ATTLIB_WORDS_PER_PAGE = 512
ATTLIB_DATA_REGION_START = 0x1000
ATTLIB_SEGMENT_POINTERS_OFFSET = 0x0800
ATTLIB_MIN_HASH = 531442
ATTLIB_MAX_HASH = 387951929
ATTLIB_PAGE_SWITCH_MARK = 0x00000000
ATTLIB_SEGMENT_END_MARK = 0xFFFFFFFF

# E3D 数据库常量
E3D_DEFAULT_PAGE_SIZE = 512
E3D_ATTRIBUTE_PAGE_SUBTYPE = 63068511  # 0x3C0A13F

# 数据页面子类型
DATA_PAGE_SUBTYPES = {
    7618377: "主要数据页面",
    13387743: "辅助数据页面",
    86284645: "索引数据页面",
    63068511: "属性数据页面",
    66156832: "扩展数据页面"
}


class DataType(IntEnum):
    """属性数据类型 (DTYP)"""
    UNKNOWN = 0
    LOGICAL = 1      # 布尔值
    REAL = 2         # 实数 (4 字节浮点)
    INTEGER = 3      # 整数 (4 字节)
    TEXT = 4         # 文本字符串
    REFERENCE = 5    # 参考号
    POSITION = 6     # 位置 (3x 实数)
    DIRECTION = 7    # 方向 (3x 实数)
    ORIENTATION = 8  # 朝向 (3x3 矩阵)


# ============================================================================
# 数据结构
# ============================================================================

@dataclass
class AttributeMetadata:
    """
    属性元数据 - 模拟 db4_get_att_dets 返回的结构
    
    对应 C++ 结构 (每条目 16 字节):
    - attr_id: int32 (属性哈希值)
    - data_type: int32 (DTYP)
    - offset: int32 (在元素数据块中的偏移)
    - length: int32 (数据长度)
    """
    attr_id: int
    data_type: DataType
    offset: int
    length: int
    is_restricted: bool = False


@dataclass
class AttlibAttrDefinition:
    """attlib.dat 中的属性定义"""
    attr_hash: int
    data_type: int
    default_flag: int
    default_value: Any = None


@dataclass
class NounAttributeBinding:
    """Noun-属性绑定 (ATNAIN)"""
    noun_id: int
    attr_id: int
    offset: int


# ============================================================================
# AttlibCache - 模拟 dword_115994E4 (shortened att types)
# ============================================================================

class AttlibCache:
    """
    属性元数据缓存 - 模拟 core.dll 中的 dword_115994E4
    
    这是一个 LRU 风格的缓存，存储最近查询的属性元数据。
    实际实现中是固定大小的数组，每条目 16 字节。
    """
    
    def __init__(self, max_entries: int = 64):
        self.max_entries = max_entries
        self.cache: Dict[int, AttributeMetadata] = {}
        self.access_order: List[int] = []
    
    def get(self, attr_id: int) -> Optional[AttributeMetadata]:
        """查询缓存"""
        if attr_id in self.cache:
            # 更新访问顺序
            self.access_order.remove(attr_id)
            self.access_order.append(attr_id)
            return self.cache[attr_id]
        return None
    
    def put(self, metadata: AttributeMetadata) -> None:
        """添加到缓存"""
        if metadata.attr_id in self.cache:
            return
        
        # 如果缓存满了，移除最旧的条目
        if len(self.cache) >= self.max_entries:
            oldest = self.access_order.pop(0)
            del self.cache[oldest]
        
        self.cache[metadata.attr_id] = metadata
        self.access_order.append(metadata.attr_id)
    
    def clear(self) -> None:
        """清空缓存"""
        self.cache.clear()
        self.access_order.clear()


# ============================================================================
# AttlibDataSource - 模拟 dword_11E1E860 (Fortran 全局表)
# ============================================================================

class AttlibDataSource:
    """
    Attlib 数据源 - 模拟 core.dll 中的 dword_11E1E860
    
    这是从 attlib.dat 解析出的属性定义表，对应 PDFINI 的解析结果。
    采用索引查找机制：Hash -> CombinedID -> Page/Slot
    """
    
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.file = None
        self.attr_definitions: Dict[int, AttlibAttrDefinition] = {}
        self.hash_to_combined_id: Dict[int, int] = {}
        self.noun_attr_bindings: List[NounAttributeBinding] = []
        self.segment_pointers: List[int] = []
        self.page_cache: Dict[int, List[int]] = {}
        
        self._open()
        self._load()
    
    def _open(self) -> None:
        """打开 attlib.dat 文件"""
        self.file = open(self.file_path, 'rb')
        self._read_segment_pointers()
    
    def _read_segment_pointers(self) -> None:
        """读取段指针表"""
        self.file.seek(ATTLIB_SEGMENT_POINTERS_OFFSET)
        for _ in range(8):
            data = self.file.read(4)
            if len(data) == 4:
                ptr = struct.unpack('>I', data)[0]  # 大端序
                self.segment_pointers.append(ptr)
    
    def _read_page(self, page_num: int) -> List[int]:
        """读取指定页"""
        if page_num in self.page_cache:
            return self.page_cache[page_num]
        
        file_offset = ATTLIB_DATA_REGION_START + page_num * ATTLIB_PAGE_SIZE
        self.file.seek(file_offset)
        page_data = self.file.read(ATTLIB_PAGE_SIZE)
        
        words = []
        for i in range(ATTLIB_WORDS_PER_PAGE):
            word_bytes = page_data[i*4:(i+1)*4]
            if len(word_bytes) == 4:
                word = struct.unpack('>I', word_bytes)[0]
                words.append(word)
        
        self.page_cache[page_num] = words
        return words
    
    def _load(self) -> None:
        """加载加载索引、定义和 Noun 绑定"""
        print(f"解析 attlib.dat: {self.file_path}")
        print(f"段指针: {self.segment_pointers}")
        self._load_atgtix()  # 首先加载索引表 (Hash -> CombinedID)
        print(f"已加载索引项: {len(self.hash_to_combined_id)}")
        self._load_atnain()  # 加载 Noun-属性绑定
        print(f"已加载 Noun-属性绑定: {len(self.noun_attr_bindings)}")
    
    def _load_atgtix(self) -> None:
        """加载 ATGTIX 段 (属性哈希到 CombinedID 的映射)"""
        if len(self.segment_pointers) < 3:
            print("错误: 段指针不足，无法定位 ATGTIX")
            return
            
        # 尝试遍历可能的索引段 (通常是段 1, 2, 3)
        for segment_idx in [1, 2, 3]:
            start_page = self.segment_pointers[segment_idx]
            if start_page == 0: continue
            
            print(f"正在从段 {segment_idx} (页 {start_page}) 加载索引...")
            page_num = start_page
            
            while True:
                words = self._read_page(page_num)
                word_idx = 0
                found_in_page = 0
                
                while word_idx < ATTLIB_WORDS_PER_PAGE:
                    word = words[word_idx]
                    word_idx += 1
                    
                    if word == ATTLIB_PAGE_SWITCH_MARK:
                        page_num += 1
                        word_idx = 0
                        words = self._read_page(page_num)
                        continue
                    
                    if word == ATTLIB_SEGMENT_END_MARK:
                        break
                    
                    # ATGTIX 条目: [Hash][CombinedID]
                    if ATTLIB_MIN_HASH <= word <= ATTLIB_MAX_HASH:
                        attr_hash = word
                        if word_idx < ATTLIB_WORDS_PER_PAGE:
                            combined_id = words[word_idx]
                            word_idx += 1
                            self.hash_to_combined_id[attr_hash] = combined_id
                            found_in_page += 1
                
                if found_in_page == 0 or words[0] == ATTLIB_SEGMENT_END_MARK:
                    break
                page_num += 1

    def _load_atnain(self) -> None:
        """加载 ATNAIN 段 (Noun-属性绑定)"""
        if len(self.segment_pointers) < 4:
            return
            
        start_page = self.segment_pointers[3] # 通常是段 3
        page_num = start_page
        
        while True:
            words = self._read_page(page_num)
            word_idx = 0
            found_in_page = 0
            
            while word_idx + 2 < ATTLIB_WORDS_PER_PAGE:
                # ATNAIN 条目格式: [AttrID, NounID, Offset]
                attr_id = words[word_idx]
                noun_id = words[word_idx + 1]
                offset = words[word_idx + 2]
                
                if attr_id == ATTLIB_PAGE_SWITCH_MARK:
                    page_num += 1
                    word_idx = 0
                    words = self._read_page(page_num)
                    continue
                
                if attr_id == ATTLIB_SEGMENT_END_MARK:
                    return
                
                if ATTLIB_MIN_HASH <= attr_id <= ATTLIB_MAX_HASH:
                    self.noun_attr_bindings.append(NounAttributeBinding(
                        noun_id=noun_id,
                        attr_id=attr_id,
                        offset=offset
                    ))
                    found_in_page += 1
                
                word_idx += 3
            
            if found_in_page == 0:
                break
            page_num += 1

    def get_attr_definition(self, attr_id: int) -> Optional[AttlibAttrDefinition]:
        """通过哈希查找属性定义"""
        if attr_id in self.attr_definitions:
            return self.attr_definitions[attr_id]
            
        if attr_id not in self.hash_to_combined_id:
            return self._scan_for_attr(attr_id)
            
        combined_id = self.hash_to_combined_id[attr_id]
        
        # 修正 CombinedID 解析
        # 扫描发现 POS (0x853B1) 在 1860+ 页，NAME (0x9C18E) 在 1463 页
        # CombinedID 可能直接就是 Word 序号
        target_page = combined_id // ATTLIB_WORDS_PER_PAGE
        slot = combined_id % ATTLIB_WORDS_PER_PAGE
        
        attr_def = self._check_slot(target_page, slot, attr_id)
        if attr_def: return attr_def
        
        # 尝试相对于段 0 的偏移 (segment_pointers[0] = 3)
        target_page_rel = self.segment_pointers[0] + combined_id // ATTLIB_WORDS_PER_PAGE
        attr_def = self._check_slot(target_page_rel, slot, attr_id)
        if attr_def: return attr_def

        return self._scan_for_attr(attr_id)

    def _check_slot(self, page_num: int, slot: int, attr_id: int) -> Optional[AttlibAttrDefinition]:
        """检查特定槽位是否匹配"""
        if page_num < 0: return None
        words = self._read_page(page_num)
        
        # 搜索邻域 (索引可能存在变长记录导致的偏移)
        for drift in range(-5, 5):
            idx = slot + drift
            if 0 <= idx < len(words) and words[idx] == attr_id:
                data_type = words[idx + 1] if idx + 1 < len(words) else 0
                default_flag = words[idx + 2] if idx + 2 < len(words) else 0
                
                attr_def = AttlibAttrDefinition(attr_hash=attr_id, data_type=data_type, default_flag=default_flag)
                self.attr_definitions[attr_id] = attr_def
                return attr_def
        return None

    def _scan_for_attr(self, attr_id: int) -> Optional[AttlibAttrDefinition]:
        """最后的手段：在所有已知段和核心页中扫描哈希"""
        # 1. 扫描所有段指针指定的起始页
        pages_to_scan = list(self.segment_pointers)
        # 2. 补充扫描发现的核心页
        pages_to_scan.extend([1463, 1688, 1703, 1860, 1865, 1873, 1923, 1868, 1869, 1872, 1874, 1875, 1879, 1882, 1883, 1885, 1894, 1899, 1904, 1905, 1907, 1909, 1910, 1911, 1915, 1916, 1917, 1919])
        
        # 去重并排序
        pages_to_scan = sorted(list(set(pages_to_scan)))
        
        for page_num in pages_to_scan:
            if page_num == 0: continue
            words = self._read_page(page_num)
            for i, word in enumerate(words):
                if word == attr_id:
                    # 发现哈希后，其后紧跟的通常是 DTYP
                    data_type = words[i + 1] if i + 1 < len(words) else 0
                    default_flag = words[i + 2] if i + 2 < len(words) else 0
                    
                    # 过滤掉显然错误的 DTYP (例如大于 10 的值通常是其他数据)
                    if not (1 <= data_type <= 8):
                        # 如果紧跟的不像 DTYP，可能 DTYP 在后面或者前面，继续寻找
                        continue
                        
                    attr_def = AttlibAttrDefinition(attr_hash=attr_id, data_type=data_type, default_flag=default_flag)
                    self.attr_definitions[attr_id] = attr_def
                    return attr_def
        return None
    
    def close(self) -> None:
        """关闭文件"""
        if self.file:
            self.file.close()


# ============================================================================
# E3DDatabase - 读取 E3D 数据库
# ============================================================================

class E3DDatabase:
    """E3D 数据库读取器"""
    
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.file_size = os.path.getsize(file_path)
        self.metadata = None
        self._read_metadata()
    
    def _read_metadata(self) -> None:
        """读取数据库元数据"""
        with open(self.file_path, 'rb') as f:
            data = f.read(512)
        
        self.metadata = {
            'db_id': struct.unpack('>I', data[0x08:0x0C])[0],
            'version': struct.unpack('>I', data[0x04:0x08])[0],
            'page_size': struct.unpack('>I', data[0x34:0x38])[0],
            'page_count': struct.unpack('>I', data[0x38:0x3C])[0],
        }
        
        # 启发式检测: 对于 desvir.dat 等旧格式，头部可能不直接给出 2048
        # 如果文件大小是 512 的倍数但不是 2048 的倍数，或者 db_id 异常
        if self.metadata['db_id'] > 1000 or self.metadata['page_size'] == 0:
            # 尝试检测
            if self.file_size % 2048 == 0:
                self.metadata['page_size'] = 2048
            else:
                self.metadata['page_size'] = 512
            self.metadata['page_count'] = self.file_size // self.metadata['page_size']
            
        print(f"数据库检测: ID={self.metadata['db_id']}, PageSize={self.metadata['page_size']}, PageCount={self.metadata['page_count']}")
    
    def read_page(self, page_num: int) -> bytes:
        """读取指定页"""
        offset = page_num * self.metadata['page_size']
        with open(self.file_path, 'rb') as f:
            f.seek(offset)
            return f.read(self.metadata['page_size'])
    
    def parse_page_header(self, page_data: bytes) -> Dict[str, Any]:
        """解析页面头"""
        page_type = struct.unpack('>I', page_data[0:4])[0]
        result = {'page_type': page_type}
        
        if page_type == 5:  # 数据页面
            type_id = struct.unpack('>I', page_data[4:8])[0]
            bucket_id = (type_id >> 13) & 0x1FFF
            result['type_id'] = type_id
            result['bucket_id'] = bucket_id
            result['subtype_name'] = DATA_PAGE_SUBTYPES.get(type_id, "未知")
        
    def find_attribute_pages(self) -> List[int]:
        """查找潜在的数据页面"""
        data_pages = []
        
        # 扫描前 2000 页
        max_scan = min(self.metadata['page_count'], 2000)
        
        for page_num in range(max_scan):
            try:
                page_data = self.read_page(page_num)
                if len(page_data) < 4: continue
                
                page_type = struct.unpack('>I', page_data[0:4])[0]
                # 在旧版中，数据页类型可能是 1 (RefArray) 或 5 (Data)
                # 元素头往往出现在这些页面中
                if page_type in [1, 5]:
                    data_pages.append(page_num)
            except Exception:
                continue
                
        return data_pages
    
    def get_element_data_by_ref(self, ref_no: str) -> Optional[Tuple[int, bytes]]:
        """
        通过参考号获取元素原始数据
        
        Args:
            ref_no: 参考号字符串 (如 "123_456")
            
        Returns:
            (noun_id, raw_data_block) 或 None
        """
        # 解析参考号
        try:
            db_id, local_id = map(int, ref_no.split('_'))
        except ValueError:
            print(f"警告: 无效的参考号格式: {ref_no}")
            return None
            
        # 查找包含此参考号的属性数据页面
        # 注意: 真正的引用解析是通过 B+ 树索引完成的，这里为了演示先扫描属性页面
        attr_pages = self.find_attribute_pages()
        
        for page_num in attr_pages:
            page_data = self.read_page(page_num)
            
            # 搜索参考号匹配的元素头部: [Ref0, Ref1, NounID]
            # 为了更鲁棒，我们在页面内逐字搜索
            for i in range(0, len(page_data) - 12, 4):
                ref0 = struct.unpack('>I', page_data[i:i+4])[0]
                ref1 = struct.unpack('>I', page_data[i+4:i+8])[0]
                
                if ref0 == db_id and ref1 == local_id:
                    # 检查下一个词是否像 NounID (通常 < 1000)
                    noun_id = struct.unpack('>I', page_data[i+8:i+12])[0]
                    if noun_id < 2000:
                        # 找到了！
                        # 真正的实现需要计算元素数据块的长度，这里我们返回整个页面的剩余部分
                        # 为了支持偏移量正确，我们需要返回一个从元素头开始的 bytearray
                        return noun_id, page_data[i:]
                    
        return None


# ============================================================================
# E3DAttributeParser - 主解析器
# ============================================================================

class E3DAttributeParser:
    """
    E3D 属性解析器 - 模拟 db4_get_att_dets 的完整逻辑
    
    实现了从 attlib.dat 加载元数据，从 E3D 数据库读取属性值的完整流程。
    """
    
    def __init__(self, attlib_path: str, db_path: Optional[str] = None):
        """
        初始化解析器
        
        Args:
            attlib_path: attlib.dat 文件路径
            db_path: E3D 数据库文件路径 (可选)
        """
        self.cache = AttlibCache()
        self.data_source = AttlibDataSource(attlib_path)
        self.database = E3DDatabase(db_path) if db_path else None
    
    def get_att_dets(self, attr_id: int, noun_id: Optional[int] = None) -> Optional[AttributeMetadata]:
        """
        获取属性详情 - 模拟 db4_get_att_dets
        
        实现两级缓存查找:
        1. 先查 C++ 缓存 (dword_115994E4)
        2. 未命中则查 Fortran 数据源 (dword_11E1E860)
        
        Args:
            attr_id: 属性哈希值
            noun_id: 所属元素的 Noun ID (用于确定物理偏移)
            
        Returns:
            属性元数据
        """
        # 1. 查询缓存 (这里我们简单化，不考虑 noun_id 的缓存冲突)
        cached = self.cache.get(attr_id)
        if cached and (noun_id is None or cached.offset != 0):
            return cached
        
        # 2. 查询 Fortran 数据源
        attr_def = self.data_source.get_attr_definition(attr_id)
        if not attr_def:
            return None
        
        # 3. 查找偏移 (如果提供了 noun_id)
        offset = 0
        if noun_id is not None:
            for binding in self.data_source.noun_attr_bindings:
                if binding.noun_id == noun_id and binding.attr_id == attr_id:
                    offset = binding.offset
                    break
        
        # 4. 构造元数据并缓存
        metadata = AttributeMetadata(
            attr_id=attr_id,
            data_type=DataType(attr_def.data_type) if attr_def.data_type in DataType._value2member_map_ else DataType.UNKNOWN,
            offset=offset,
            length=self._get_type_length(attr_def.data_type)
        )
        
        self.cache.put(metadata)
        return metadata
    
    def get_attribute_value_by_ref(self, ref_no: str, attr_id: int) -> Any:
        """
        通过参考号和属性 ID 读取属性值
        
        Args:
            ref_no: 参考号 (123_456)
            attr_id: 属性哈希 (如 639374)
            
        Returns:
            解析后的属性值
        """
        if not self.database:
            print("错误: 未提供数据库文件")
            return None
            
        # 1. 定位元素并获取 NounID
        result = self.database.get_element_data_by_ref(ref_no)
        if not result:
            print(f"✗ 元素 {ref_no} 未找到")
            return None
            
        noun_id, page_data = result
        
        # 2. 获取属性元数据 (包含 Offset)
        metadata = self.get_att_dets(attr_id, noun_id)
        if not metadata:
            print(f"✗ 属性 {attr_id} 未定义")
            return None
            
        if metadata.offset == 0:
            print(f"✗ 属性 {attr_id} 在 Noun {noun_id} 中没有偏移量定义 (ATNAIN)")
            return None
            
        # 3. 读取并解码值
        # 页面数据中元素起始位置之后加上 Offset
        # 注意: 这里的读取逻辑需要根据元素在页面中的实际物理起始地址微调
        # 这里假设 offset 是相对于页面起始的 Word 偏移 (待进一步验证)
        byte_offset = metadata.offset * 4
        return self.read_attribute_value(page_data, byte_offset, metadata.data_type)

    def _get_type_length(self, data_type: int) -> int:
        """获取数据类型的默认长度"""
        lengths = {
            DataType.LOGICAL: 4,
            DataType.REAL: 4,
            DataType.INTEGER: 4,
            DataType.TEXT: 0,  # 变长
            DataType.REFERENCE: 4,
            DataType.POSITION: 12,
            DataType.DIRECTION: 12,
            DataType.ORIENTATION: 36,
        }
        return lengths.get(data_type, 4)
    
    def read_attribute_value(self, page_data: bytes, offset: int, data_type: DataType) -> Any:
        """
        从页面数据中读取属性值
        
        Args:
            page_data: 页面数据
            offset: 偏移量
            data_type: 数据类型
            
        Returns:
            解析后的属性值
        """
        if offset >= len(page_data):
            return None
        
        if data_type == DataType.LOGICAL:
            value = struct.unpack('>I', page_data[offset:offset+4])[0]
            return bool(value)
        
        elif data_type == DataType.INTEGER:
            return struct.unpack('>i', page_data[offset:offset+4])[0]
        
        elif data_type == DataType.REAL:
            return struct.unpack('>f', page_data[offset:offset+4])[0]
        
        elif data_type == DataType.REFERENCE:
            return struct.unpack('>I', page_data[offset:offset+4])[0]
        
        elif data_type == DataType.POSITION or data_type == DataType.DIRECTION:
            x = struct.unpack('>f', page_data[offset:offset+4])[0]
            y = struct.unpack('>f', page_data[offset+4:offset+8])[0]
            z = struct.unpack('>f', page_data[offset+8:offset+12])[0]
            return (x, y, z)
        
        elif data_type == DataType.TEXT:
            # 文本格式: 4字节长度 + 内容
            length = struct.unpack('>I', page_data[offset:offset+4])[0]
            text_data = page_data[offset+4:offset+4+length*4]
            # 将 32-bit words 转换为字符串
            chars = []
            for i in range(0, len(text_data), 4):
                word = struct.unpack('>I', text_data[i:i+4])[0]
                chars.append(chr(word) if 32 <= word < 127 else '?')
            return ''.join(chars)
        
        else:
            return struct.unpack('>I', page_data[offset:offset+4])[0]
    
    def get_all_attr_definitions(self) -> Dict[int, AttlibAttrDefinition]:
        """获取所有属性定义"""
        return self.data_source.attr_definitions
    
    def print_summary(self) -> None:
        """打印摘要信息"""
        print("=" * 80)
        print("E3D 属性解析器摘要")
        print("=" * 80)
        print(f"属性定义数: {len(self.data_source.attr_definitions)}")
        print(f"缓存大小: {len(self.cache.cache)}/{self.cache.max_entries}")
        
        if self.database:
            print(f"\n数据库: {self.database.file_path}")
            print(f"  页面大小: {self.database.metadata['page_size']} 字节")
            print(f"  总页数: {self.database.metadata['page_count']}")
        
        # 按数据类型统计
        type_counts: Dict[int, int] = {}
        for attr in self.data_source.attr_definitions.values():
            type_counts[attr.data_type] = type_counts.get(attr.data_type, 0) + 1
        
        print("\n属性类型分布:")
        type_names = {1: "LOG", 2: "REAL", 3: "INT", 4: "TEXT", 5: "REF"}
        for dtype, count in sorted(type_counts.items()):
            name = type_names.get(dtype, f"TYPE_{dtype}")
            print(f"  {name}: {count}")
    
    def close(self) -> None:
        """关闭资源"""
        self.data_source.close()


# ============================================================================
# 命令行接口
# ============================================================================

def main():
    """主函数"""
    if len(sys.argv) < 2:
        print(__doc__)
        print("\n使用示例:")
        print("  python e3d_attribute_parser.py data/attlib.dat")
        print("  python e3d_attribute_parser.py data/attlib.dat data/ams7330_0001")
        sys.exit(1)
    
    attlib_path = sys.argv[1]
    db_path = sys.argv[2] if len(sys.argv) > 2 else None
    
    # 创建解析器
    parser = E3DAttributeParser(attlib_path, db_path)
    
    # 打印摘要
    parser.print_summary()
    
    # 情况 1: 只提供了 attlib.dat
    if not db_path:
        # 测试一些常见属性
        test_attrs = {
            "POS": 545713,
            "ORI": 538503,
            "NAME": 639374,
            "TYPE": 642215,
        }
        
        print("\n" + "=" * 80)
        print("属性查询测试 (模拟 db4_get_att_dets)")
        print("=" * 80)
        
        for name, attr_id in test_attrs.items():
            metadata = parser.get_att_dets(attr_id)
            if metadata:
                print(f"✓ {name} (hash: {attr_id})")
                print(f"    DTYP: {metadata.data_type.name}")
                print(f"    Length: {metadata.length} bytes")
            else:
                print(f"✗ {name} (hash: {attr_id}) - NOT FOUND")
    
    # 情况 2: 提供了数据库文件和参考号
    else:
        ref_no = sys.argv[3] if len(sys.argv) > 3 else None
        if not ref_no:
            print("\n提示: 请提供参考号以测试数值读取 (示例: 123_456)")
        else:
            print("\n" + "=" * 80)
            print(f"属性值读取测试: 元素 {ref_no}")
            print("=" * 80)
            
            # 测试读取 NAME (639374) 和 POS (545713)
            for name, attr_id in [("NAME", 639374), ("POS", 545713)]:
                val = parser.get_attribute_value_by_ref(ref_no, attr_id)
                print(f"  {name}: {val}")

    parser.close()


if __name__ == '__main__':
    main()
