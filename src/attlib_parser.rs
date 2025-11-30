use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const PAGE_SIZE: usize = 2048;
const WORDS_PER_PAGE: usize = 512;
const MIN_HASH: u32 = 531442;      // 0x81BF2 - IDA: ATTLIB_Load_Index_ATGTIX
const MAX_HASH: u32 = 387951929;   // 0x171FAD39 - IDA: ATTLIB_Load_Index_ATGTIX
const PAGE_SWITCH_MARK: u32 = 0x00000000;
const SEGMENT_END_MARK: u32 = 0xFFFFFFFF;
const DATA_REGION_START: u64 = 0x1000;
const SEGMENT_POINTERS_OFFSET: u64 = 0x0800;

/// PDMS 哈希范围常量
/// 基于 db_tool.rs 中的 db1_dehash_uncached 函数
const HASH_BASE_OFFSET: u32 = 0x81BF1;     // 531441 - 基础偏移量
const HASH_UDA_THRESHOLD: u32 = 0x171FAD39; // 387951929 - UDA 属性阈值

/// 属性数据类型枚举
/// 基于 IDA Pro 分析的 ATTLIB_Load_Def_ATGTDF (0x10852E20)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum AttlibDataType {
    /// 逻辑/布尔类型
    Log = 1,
    /// 双精度浮点类型
    Real = 2,
    /// 32 位整数类型
    Int = 3,
    /// 文本类型（27 进制编码）
    Text = 4,
}

impl AttlibDataType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::Log),
            2 => Some(Self::Real),
            3 => Some(Self::Int),
            4 => Some(Self::Text),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Log => "LOG",
            Self::Real => "REAL",
            Self::Int => "INT",
            Self::Text => "TEXT",
        }
    }
}

struct WordCursor {
    page_num: u32,
    word_idx: usize,
    words: Vec<u32>,
}

impl WordCursor {
    fn new(parser: &mut AttlibParser, start_page: u32) -> std::io::Result<Self> {
        let words = parser.read_page(start_page)?;
        Ok(Self {
            page_num: start_page,
            word_idx: 0,
            words,
        })
    }

    fn next_word(&mut self, parser: &mut AttlibParser) -> std::io::Result<u32> {
        if self.word_idx >= WORDS_PER_PAGE {
            self.advance_page(parser)?;
        }
        let word = self.words[self.word_idx];
        self.word_idx += 1;
        Ok(word)
    }

    fn advance_page(&mut self, parser: &mut AttlibParser) -> std::io::Result<()> {
        self.page_num = self.page_num.checked_add(1).unwrap_or(self.page_num);
        self.words = parser.read_page(self.page_num)?;
        self.word_idx = 0;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttlibAttrIndex {
    pub attr_hash: u32,
    pub combined: u32,
}

impl AttlibAttrIndex {
    pub fn record_num(&self) -> u32 {
        self.combined / 512
    }

    pub fn slot_offset(&self) -> u32 {
        self.combined % 512
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttlibAttrDefinition {
    pub attr_hash: u32,
    pub data_type: u32,
    pub default_flag: u32,
    pub default_value: AttlibDefaultValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttlibDefaultValue {
    None,
    /// 标量值（LOG/INT/REAL）
    Scalar(u32),
    /// 文本值（27 进制编码的原始数据）
    Text(Vec<u32>),
}

impl AttlibDefaultValue {
    /// 将标量值解释为布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Scalar(v) => Some(*v != 0),
            _ => None,
        }
    }

    /// 将标量值解释为整数
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Scalar(v) => Some(*v as i32),
            _ => None,
        }
    }

    /// 将标量值解释为浮点数（IEEE 754 单精度）
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Self::Scalar(v) => Some(f32::from_bits(*v) as f64),
            _ => None,
        }
    }

    /// 将文本值解码为字符串（27 进制解码）
    pub fn as_text(&self) -> Option<String> {
        match self {
            Self::Text(words) => Some(decode_base27(words)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttlibAttribute {
    pub hash: u32,
    pub name: String,
    pub data_type: AttlibDataType,
    pub default_value: AttlibDefaultValue,
    /// 默认值的字符串表示
    pub default_text: Option<String>,
}

/// 27 进制解码器（用于 TEXT 类型默认值）
/// 基于 db_tool.rs 的解码逻辑
/// 每个 32 位字编码最多 6 个字符
pub fn decode_base27(words: &[u32]) -> String {
    let mut result = String::new();
    for &word in words {
        let mut k = word as i32;
        let mut chars = Vec::with_capacity(6);
        while k > 0 {
            // 64 = '@', 65 = 'A', ... 字符映射
            let c = (k % 27 + 64) as u8 as char;
            chars.push(c);
            k /= 27;
        }
        // 字符已经是正确顺序，直接追加
        for c in chars {
            result.push(c);
        }
    }
    result
}

/// 27 进制编码器（用于计算属性哈希）
/// 基于 db_tool.rs 中的编码逻辑
pub fn encode_base27(name: &str) -> u32 {
    let mut k: u32 = 0;
    // 从右到左编码，每个字符的权重递增
    for c in name.chars().take(6).collect::<Vec<_>>().into_iter().rev() {
        let idx = match c.to_ascii_uppercase() {
            '@' => 0,
            c @ 'A'..='Z' => (c as u32) - ('A' as u32) + 1,
            ' ' => 0,
            _ => 0,
        };
        k = k * 27 + idx;
    }
    k + HASH_BASE_OFFSET + 1
}

/// 从哈希值解码属性名称
/// 基于 db_tool.rs 中的 db1_dehash_uncached 函数
pub fn decode_hash_to_name(hash: u32) -> String {
    let mut result = String::new();
    
    if hash > HASH_UDA_THRESHOLD {
        // UDA 属性：64 进制编码，前缀 ':'
        let mut k = ((hash - HASH_UDA_THRESHOLD) % 0x1000000) as i32;
        result.push(':');
        for _ in 0..6 {
            if k <= 0 {
                break;
            }
            result.push((k % 64 + 32) as u8 as char);
            k /= 64;
        }
    } else if hash > HASH_BASE_OFFSET {
        // 普通属性：27 进制编码
        let mut k = (hash - HASH_BASE_OFFSET) as i32;
        while k > 0 {
            // 64 = '@'，65 = 'A'，66 = 'B'...
            result.push((k % 27 + 64) as u8 as char);
            k /= 27;
        }
    }
    
    result
}

pub struct AttlibParser {
    file: File,
    /// ATGTIX 属性索引表
    pub attr_index: HashMap<u32, AttlibAttrIndex>,
    /// ATGTDF 属性定义表
    pub attr_definitions: HashMap<u32, AttlibAttrDefinition>,
    /// 段指针表（8 个段）
    /// [0]=ATGTDF-1, [2]=ATGTIX-1, [4]=ATGTDF-2, [6]=ATGTIX-2
    segment_pointers: [u32; 8],
    /// 页面缓存
    page_cache: HashMap<u32, Vec<u32>>,
    /// 解析统计
    pub stats: AttlibStats,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AttlibStats {
    pub atgtix_count: usize,
    pub atgtdf_count: usize,
    pub text_defaults: usize,
    pub scalar_defaults: usize,
    pub no_defaults: usize,
}

/// ATGTSX 语法表条目（类型-属性映射）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttlibSyntaxEntry {
    /// 属性哈希
    pub attr_hash: u32,
    /// 类型哈希
    pub noun_hash: u32,
    /// 额外信息（如数组索引）
    pub extra_info: u32,
}

impl AttlibSyntaxEntry {
    /// 获取属性名称
    pub fn attr_name(&self) -> String {
        decode_hash_to_name(self.attr_hash)
    }
    
    /// 获取类型名称
    pub fn noun_name(&self) -> String {
        decode_hash_to_name(self.noun_hash)
    }
}

/// 类型-属性映射表
pub struct NounAttributeMap {
    /// noun_hash → Vec<attr_hash>
    pub mapping: HashMap<u32, Vec<u32>>,
}

impl NounAttributeMap {
    /// 从语法表构建映射
    pub fn from_syntax(entries: &[AttlibSyntaxEntry]) -> Self {
        let mut mapping: HashMap<u32, Vec<u32>> = HashMap::new();
        
        for entry in entries {
            mapping
                .entry(entry.noun_hash)
                .or_default()
                .push(entry.attr_hash);
        }
        
        Self { mapping }
    }
    
    /// 获取类型的所有属性哈希
    pub fn get_attributes(&self, noun_hash: u32) -> Vec<u32> {
        self.mapping.get(&noun_hash).cloned().unwrap_or_default()
    }
    
    /// 获取类型的所有属性名称
    pub fn get_attribute_names(&self, noun_hash: u32) -> Vec<String> {
        self.get_attributes(noun_hash)
            .iter()
            .map(|&h| decode_hash_to_name(h))
            .collect()
    }
    
    /// 通过类型名称获取属性
    pub fn get_attributes_by_name(&self, noun_name: &str) -> Vec<String> {
        let noun_hash = encode_base27(noun_name);
        self.get_attribute_names(noun_hash)
    }
    
    /// 获取所有类型名称
    pub fn all_nouns(&self) -> Vec<String> {
        self.mapping.keys().map(|&h| decode_hash_to_name(h)).collect()
    }
    
    /// 获取类型数量
    pub fn noun_count(&self) -> usize {
        self.mapping.len()
    }
}

impl AttlibParser {
    pub fn new(file_path: &str) -> std::io::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut segment_pointers = [0u32; 8];

        // 读取段指针表 (0x0800)
        file.seek(SeekFrom::Start(SEGMENT_POINTERS_OFFSET))?;
        let mut ptr_buf = [0u8; 32];
        file.read_exact(&mut ptr_buf)?;

        for i in 0..8 {
            let bytes = &ptr_buf[i * 4..(i + 1) * 4];
            segment_pointers[i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }

        eprintln!("段指针表:");
        for (i, ptr) in segment_pointers.iter().enumerate() {
            eprintln!("  段 {}: 0x{:08X} (页号: {})", i, ptr, ptr);
        }

        Ok(AttlibParser {
            file,
            attr_index: HashMap::new(),
            attr_definitions: HashMap::new(),
            segment_pointers,
            page_cache: HashMap::new(),
            stats: AttlibStats::default(),
        })
    }

    /// 获取文件头信息
    pub fn read_header(&mut self) -> std::io::Result<String> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut header_buf = [0u8; 256];
        self.file.read_exact(&mut header_buf)?;
        
        // UTF-16LE 解码
        let mut chars = Vec::new();
        for i in (0..header_buf.len()).step_by(4) {
            if i + 3 < header_buf.len() {
                let c = u16::from_le_bytes([header_buf[i], header_buf[i + 1]]);
                if c > 0 && c < 128 {
                    chars.push(c as u8 as char);
                }
            }
        }
        Ok(chars.into_iter().collect())
    }

    pub fn load_all(&mut self) -> std::io::Result<()> {
        eprintln!("\n=== attlib.dat 解析开始 ===");
        
        // 从页 0 开始加载属性定义
        eprintln!("\n加载属性定义（从页 0 开始）...");
        self.load_definitions_from_page(0)?;
        
        // 加载 ATGTIX-1
        let atgtix1_page = self.segment_pointers[2];
        eprintln!("\n加载 ATGTIX-1 段（属性索引），起始页: {}", atgtix1_page);
        self.load_atgtix_from_page(atgtix1_page)?;
        
        // 加载 ATGTIX-2（如果有）
        let atgtix2_page = self.segment_pointers[6];
        if atgtix2_page > 0 {
            eprintln!("\n加载 ATGTIX-2 段（属性索引），起始页: {}", atgtix2_page);
            self.load_atgtix_from_page(atgtix2_page)?;
        }
        
        self.stats.atgtix_count = self.attr_index.len();
        self.stats.atgtdf_count = self.attr_definitions.len();
        
        eprintln!("\n=== 解析完成 ===");
        eprintln!("  ATGTIX 索引: {} 条", self.stats.atgtix_count);
        eprintln!("  ATGTDF 定义: {} 条", self.stats.atgtdf_count);
        eprintln!("  TEXT 默认值: {} 条", self.stats.text_defaults);
        eprintln!("  标量默认值: {} 条", self.stats.scalar_defaults);
        eprintln!("  无默认值: {} 条", self.stats.no_defaults);
        
        Ok(())
    }
    
    /// 加载 ATGTSX 语法表（类型-属性映射）
    /// 返回类型-属性映射表
    pub fn load_syntax(&mut self) -> std::io::Result<NounAttributeMap> {
        let atgtsx_page = self.segment_pointers[3];
        eprintln!("\n加载 ATGTSX 段（语法表），起始页: {}", atgtsx_page);
        
        let entries = self.load_atgtsx_from_page(atgtsx_page)?;
        let map = NounAttributeMap::from_syntax(&entries);
        
        eprintln!("  语法表加载完成: {} 个类型, {} 条映射", 
            map.noun_count(), entries.len());
        
        Ok(map)
    }
    
    /// 从指定页加载 ATGTSX 语法表
    fn load_atgtsx_from_page(&mut self, start_page: u32) -> std::io::Result<Vec<AttlibSyntaxEntry>> {
        let mut entries = Vec::new();
        let mut cursor = WordCursor::new(self, start_page)?;
        let mut record_count = 0;
        
        loop {
            let word = cursor.next_word(self)?;
            
            // 页切换标记
            if word == PAGE_SWITCH_MARK {
                cursor.advance_page(self)?;
                continue;
            }
            
            // 段结束标记
            if word == SEGMENT_END_MARK || word == 0 {
                eprintln!("  ATGTSX 加载完成，共 {} 条记录", record_count);
                return Ok(entries);
            }
            
            // 读取类型哈希和额外信息
            let attr_hash = word;
            let noun_hash = cursor.next_word(self)?;
            let extra_info = cursor.next_word(self)?;
            
            entries.push(AttlibSyntaxEntry {
                attr_hash,
                noun_hash,
                extra_info,
            });
            
            if record_count < 5 {
                let attr_name = decode_hash_to_name(attr_hash);
                let noun_name = decode_hash_to_name(noun_hash);
                eprintln!("    [{}] attr={:6} noun={:6} extra={}", 
                    record_count, attr_name, noun_name, extra_info);
            }
            
            record_count += 1;
        }
    }
    
    /// 从指定页开始加载属性定义
    /// 属性定义结构: [hash, data_type, default_flag, ...]
    fn load_definitions_from_page(&mut self, start_page: u32) -> std::io::Result<()> {
        let mut page_num = start_page;
        let mut record_count = 0;
        
        loop {
            let words = self.read_page(page_num)?;
            let mut i = 0;
            
            while i < WORDS_PER_PAGE {
                let word = words[i];
                
                // 页切换标记
                if word == PAGE_SWITCH_MARK {
                    page_num += 1;
                    break;
                }
                
                // 段结束标记
                if word == SEGMENT_END_MARK {
                    eprintln!("  属性定义加载完成，共 {} 条记录", record_count);
                    return Ok(());
                }
                
                // 哈希范围检查
                if word < MIN_HASH || word > MAX_HASH {
                    i += 1;
                    continue;
                }
                
                // 检查是否有足够的字段
                if i + 2 >= WORDS_PER_PAGE {
                    page_num += 1;
                    break;
                }
                
                let attr_hash = word;
                let data_type = words[i + 1];
                let default_flag = words[i + 2];
                
                // 验证数据类型有效性
                if data_type < 1 || data_type > 4 {
                    i += 1;
                    continue;
                }
                
                // 验证默认值标志有效性
                if default_flag != 1 && default_flag != 2 {
                    i += 1;
                    continue;
                }
                
                // 解析默认值
                let (default_value, consumed) = if default_flag == 1 {
                    self.stats.no_defaults += 1;
                    (AttlibDefaultValue::None, 3)
                } else if data_type == 4 {
                    // TEXT 类型: 先读长度，再读数据
                    if i + 3 < WORDS_PER_PAGE {
                        let length = words[i + 3] as usize;
                        let end = (i + 4 + length).min(WORDS_PER_PAGE);
                        let actual_len = end - (i + 4);
                        let text_data: Vec<u32> = words[i + 4..end].to_vec();
                        self.stats.text_defaults += 1;
                        (AttlibDefaultValue::Text(text_data), 4 + actual_len)
                    } else {
                        (AttlibDefaultValue::None, 3)
                    }
                } else {
                    // 标量类型
                    if i + 3 < WORDS_PER_PAGE {
                        self.stats.scalar_defaults += 1;
                        (AttlibDefaultValue::Scalar(words[i + 3]), 4)
                    } else {
                        (AttlibDefaultValue::None, 3)
                    }
                };
                
                self.attr_definitions.insert(
                    attr_hash,
                    AttlibAttrDefinition {
                        attr_hash,
                        data_type,
                        default_flag,
                        default_value,
                    },
                );
                
                if record_count < 5 {
                    let name = decode_hash_to_name(attr_hash);
                    let type_name = AttlibDataType::from_u32(data_type)
                        .map(|t| t.name())
                        .unwrap_or("UNK");
                    eprintln!("    [{}] hash=0x{:08X} name={:6} type={} flag={}", 
                        record_count, attr_hash, name, type_name, default_flag);
                }
                
                record_count += 1;
                i += consumed;
            }
            
            // 安全检查：防止无限循环
            if page_num > 3000 {
                eprintln!("  警告: 达到页面上限，停止扫描");
                break;
            }
        }
        
        eprintln!("  属性定义加载完成，共 {} 条记录", record_count);
        Ok(())
    }

    /// 读取指定页号的页面（页号是相对于 DATA_REGION_START 的）
    fn read_page(&mut self, page_num: u32) -> std::io::Result<Vec<u32>> {
        if let Some(cached) = self.page_cache.get(&page_num) {
            return Ok(cached.clone());
        }

        let file_offset = DATA_REGION_START + (page_num as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(file_offset))?;

        let mut page_buf = vec![0u8; PAGE_SIZE];
        self.file.read_exact(&mut page_buf)?;

        let mut words = Vec::with_capacity(WORDS_PER_PAGE);
        for i in 0..WORDS_PER_PAGE {
            let bytes = &page_buf[i * 4..(i + 1) * 4];
            let word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            words.push(word);
        }

        self.page_cache.insert(page_num, words.clone());
        Ok(words)
    }

    fn load_atgtix_from_page(&mut self, start_page: u32) -> std::io::Result<()> {
        let mut cursor = WordCursor::new(self, start_page)?;
        let mut record_count = 0;

        loop {
            let word = cursor.next_word(self)?;

            // 页切换标记
            if word == PAGE_SWITCH_MARK {
                cursor.advance_page(self)?;
                continue;
            }

            // 段结束标记
            if word == SEGMENT_END_MARK {
                eprintln!("  ATGTIX 加载完成，共 {} 条记录", record_count);
                return Ok(());
            }

            // 哈希范围检查（基于 IDA 分析）
            if word < MIN_HASH || word > MAX_HASH {
                continue;
            }

            let attr_hash = word;
            let combined = cursor.next_word(self)?;
            
            // 解码属性名称
            let name = decode_hash_to_name(attr_hash);

            self.attr_index.insert(
                attr_hash,
                AttlibAttrIndex {
                    attr_hash,
                    combined,
                },
            );

            if record_count < 5 {
                eprintln!("    [{}] hash=0x{:08X} name={:6} combined={}", 
                    record_count, attr_hash, name, combined);
            }
            record_count += 1;
        }
    }

    fn load_atgtdf_from_page(&mut self, start_page: u32) -> std::io::Result<()> {
        let mut cursor = WordCursor::new(self, start_page)?;
        let mut record_count = 0;

        loop {
            let word = cursor.next_word(self)?;

            // 页切换标记
            if word == PAGE_SWITCH_MARK {
                cursor.advance_page(self)?;
                continue;
            }

            // 段结束标记
            if word == SEGMENT_END_MARK {
                eprintln!("  ATGTDF 加载完成，共 {} 条记录", record_count);
                return Ok(());
            }

            // 哈希范围检查
            if word < MIN_HASH || word > MAX_HASH {
                continue;
            }

            let attr_hash = word;
            let data_type_raw = cursor.next_word(self)?;
            let default_flag = cursor.next_word(self)?;

            let default_value = self.read_default_value(&mut cursor, data_type_raw, default_flag)?;
            
            // 更新统计
            match &default_value {
                AttlibDefaultValue::None => self.stats.no_defaults += 1,
                AttlibDefaultValue::Scalar(_) => self.stats.scalar_defaults += 1,
                AttlibDefaultValue::Text(_) => self.stats.text_defaults += 1,
            }

            self.attr_definitions.insert(
                attr_hash,
                AttlibAttrDefinition {
                    attr_hash,
                    data_type: data_type_raw,
                    default_flag,
                    default_value,
                },
            );

            if record_count < 5 {
                let name = decode_hash_to_name(attr_hash);
                let type_name = AttlibDataType::from_u32(data_type_raw)
                    .map(|t| t.name())
                    .unwrap_or("UNK");
                eprintln!("    [{}] hash=0x{:08X} name={:6} type={}", 
                    record_count, attr_hash, name, type_name);
            }
            record_count += 1;
        }
    }

    /// 读取默认值，处理跨页情况
    fn read_default_value(
        &mut self,
        cursor: &mut WordCursor,
        data_type: u32,
        default_flag: u32,
    ) -> std::io::Result<AttlibDefaultValue> {
        if default_flag == 1 {
            return Ok(AttlibDefaultValue::None);
        }

        if default_flag != 2 {
            return Ok(AttlibDefaultValue::None);
        }

        if data_type == 4 {
            // TEXT 类型：先读长度，再读数据
            let length = cursor.next_word(self)? as usize;

            let mut text_data = Vec::new();
            for _ in 0..length {
                text_data.push(cursor.next_word(self)?);
            }

            Ok(AttlibDefaultValue::Text(text_data))
        } else {
            // 标量类型
            let scalar = cursor.next_word(self)?;

            Ok(AttlibDefaultValue::Scalar(scalar))
        }
    }

    pub fn get_attribute(&self, hash: u32) -> Option<&AttlibAttrDefinition> {
        self.attr_definitions.get(&hash)
    }

    pub fn get_all_attributes(&self) -> Vec<&AttlibAttrDefinition> {
        self.attr_definitions.values().collect()
    }
    
    /// 通过属性名称查找定义
    pub fn get_attribute_by_name(&self, name: &str) -> Option<&AttlibAttrDefinition> {
        let hash = encode_base27(name);
        self.attr_definitions.get(&hash)
    }
    
    /// 获取完整的属性信息（合并索引和定义）
    pub fn get_full_attribute(&self, hash: u32) -> Option<AttlibAttribute> {
        let def = self.attr_definitions.get(&hash)?;
        let name = decode_hash_to_name(hash);
        let data_type = AttlibDataType::from_u32(def.data_type)?;
        
        let default_text = match &def.default_value {
            AttlibDefaultValue::None => None,
            AttlibDefaultValue::Scalar(v) => match data_type {
                AttlibDataType::Log => Some(if *v != 0 { "TRUE" } else { "FALSE" }.to_string()),
                AttlibDataType::Int => Some((*v as i32).to_string()),
                AttlibDataType::Real => Some(format!("{:.6}", f32::from_bits(*v))),
                AttlibDataType::Text => None,
            },
            AttlibDefaultValue::Text(words) => Some(decode_base27(words)),
        };
        
        Some(AttlibAttribute {
            hash,
            name,
            data_type,
            default_value: def.default_value.clone(),
            default_text,
        })
    }
    
    /// 列出所有属性（带名称解码）
    pub fn list_all_attributes(&self) -> Vec<AttlibAttribute> {
        self.attr_definitions
            .keys()
            .filter_map(|&hash| self.get_full_attribute(hash))
            .collect()
    }
    
    /// 导出为 JSON
    pub fn export_json(&self) -> serde_json::Result<String> {
        let attrs = self.list_all_attributes();
        serde_json::to_string_pretty(&attrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_base27_encode_decode() {
        let names = ["NAME", "TYPE", "OWNER", "REFNO", "ZONE", "EQUI"];
        for name in names {
            let hash = encode_base27(name);
            let decoded = decode_hash_to_name(hash);
            assert_eq!(name, decoded, "编码解码往返失败: {}", name);
        }
    }
    
    #[test]
    fn test_known_hashes() {
        // 已知的属性哈希值
        assert_eq!(encode_base27("NAME"), 639374);
        assert_eq!(encode_base27("TYPE"), 642215);
    }
}
