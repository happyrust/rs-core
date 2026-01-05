use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const PAGE_SIZE: usize = 2048;
const WORDS_PER_PAGE: usize = 512;
const MIN_HASH: u32 = 531442;
const MAX_HASH: u32 = 387951929;
const PAGE_SWITCH_MARK: u32 = 0x00000000;
const SEGMENT_END_MARK: u32 = 0xFFFFFFFF;
const DATA_REGION_START: u64 = 0x1000;
const SEGMENT_POINTERS_OFFSET: u64 = 0x0800;

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
    Scalar(u32),
    Text(Vec<u32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttlibAttribute {
    pub hash: u32,
    pub name: String,
    pub data_type: u32,
    pub default_value: AttlibDefaultValue,
}

/// ATGTSX 语法表条目（类型-属性映射）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttlibSyntaxEntry {
    /// 属性哈希
    pub attr_hash: u32,
    /// 类型（Noun）哈希
    pub noun_hash: u32,
    /// 额外信息（如数组索引）
    pub extra_info: u32,
}

/// Noun -> 属性哈希列表映射
pub type NounAttrMapping = HashMap<u32, Vec<u32>>;

/// Base27 常量
const BASE27_OFFSET: u32 = 0x81BF1;

/// 将哈希值解码为属性/类型名称
pub fn decode_base27(hash: u32) -> String {
    if hash < MIN_HASH || hash > MAX_HASH {
        return String::new();
    }

    let mut k = (hash - BASE27_OFFSET) as i64;
    let mut chars = Vec::new();

    while k > 0 {
        let c = (k % 27) as u8;
        if c == 0 {
            chars.push(b' ');
        } else {
            chars.push(c + 64); // A=65, B=66, ...
        }
        k /= 27;
    }

    String::from_utf8(chars).unwrap_or_default()
}

/// 将名称编码为哈希值
pub fn encode_base27(name: &str) -> u32 {
    let mut hash = BASE27_OFFSET;
    let mut mul = 1u32;

    for ch in name.chars().take(6) {
        let v = if ch == ' ' {
            0
        } else {
            (ch.to_ascii_uppercase() as u32).saturating_sub(64)
        };
        hash = hash.wrapping_add(mul.wrapping_mul(v));
        mul = mul.wrapping_mul(27);
    }

    hash
}

pub struct AttlibParser {
    file: File,
    attr_index: HashMap<u32, AttlibAttrIndex>,
    attr_definitions: HashMap<u32, AttlibAttrDefinition>,
    segment_pointers: [u32; 8],
    page_cache: HashMap<u32, Vec<u32>>,
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
        })
    }

    pub fn load_all(&mut self) -> std::io::Result<()> {
        eprintln!("\n加载 ATGTDF 段（属性定义）");
        self.load_atgtdf()?;
        eprintln!("\n加载 ATGTIX 段（属性索引）");
        self.load_atgtix()?;
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

    fn load_atgtix(&mut self) -> std::io::Result<()> {
        // 直接从页 0 开始（0x1000），忽略段指针
        let start_page = 0;
        eprintln!("  起始页号: {} (直接从 0x1000 开始)", start_page);
        eprintln!("  段指针表: {:?}", self.segment_pointers);

        let mut cursor = WordCursor::new(self, start_page)?;
        let mut record_count = 0;

        loop {
            let word = cursor.next_word(self)?;

            if word == PAGE_SWITCH_MARK {
                cursor.advance_page(self)?;
                continue;
            }

            if word == SEGMENT_END_MARK {
                eprintln!("  ATGTIX 加载完成，共 {} 条记录", record_count);
                return Ok(());
            }

            if word < MIN_HASH || word > MAX_HASH {
                continue;
            }

            let attr_hash = word;
            let combined = cursor.next_word(self)?;

            self.attr_index.insert(
                attr_hash,
                AttlibAttrIndex {
                    attr_hash,
                    combined,
                },
            );

            if record_count < 5 {
                eprintln!("    [{}] hash=0x{:08X}", record_count, attr_hash);
            }
            record_count += 1;
        }
    }

    fn load_atgtdf(&mut self) -> std::io::Result<()> {
        // ATGTDF 段在 ATGTIX 段之后，从页 0 开始扫描找到 ATGTIX 段结束
        let start_page = 0;
        eprintln!("  从页 0 开始扫描，找到 ATGTIX 段结束后的 ATGTDF 段");
        eprintln!("  段指针表: {:?}", self.segment_pointers);

        let mut cursor = WordCursor::new(self, start_page)?;
        let mut record_count = 0;

        loop {
            let word = cursor.next_word(self)?;

            if word == PAGE_SWITCH_MARK {
                cursor.advance_page(self)?;
                continue;
            }

            if word == SEGMENT_END_MARK {
                eprintln!("  ATGTDF 加载完成，共 {} 条记录", record_count);
                return Ok(());
            }

            if word < MIN_HASH || word > MAX_HASH {
                continue;
            }

            let attr_hash = word;
            let data_type = cursor.next_word(self)?;
            let default_flag = cursor.next_word(self)?;

            let default_value = self.read_default_value(&mut cursor, data_type, default_flag)?;

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
                eprintln!("    [{}] hash=0x{:08X}", record_count, attr_hash);
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

    /// 加载 ATGTSX 段（类型-属性语法映射）
    ///
    /// ATGTSX 段存储了 NOUN（类型）和 属性 的映射关系，
    /// 每条记录包含 3 个 u32: [attr_hash, noun_hash, extra_info]
    pub fn load_atgtsx(&mut self) -> std::io::Result<Vec<AttlibSyntaxEntry>> {
        // 从段指针表获取 ATGTSX 起始页（索引 3）
        let start_page = self.segment_pointers[3];
        if start_page == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "ATGTSX 段指针为空",
            ));
        }

        eprintln!("加载 ATGTSX 段，起始页: {}", start_page);

        let mut cursor = WordCursor::new(self, start_page)?;
        let mut entries = Vec::new();

        loop {
            let word = cursor.next_word(self)?;

            // 处理页面切换
            if word == PAGE_SWITCH_MARK {
                cursor.advance_page(self)?;
                continue;
            }

            // 处理段结束
            if word == SEGMENT_END_MARK || word == 0 {
                break;
            }

            // 跳过无效哈希
            if word < MIN_HASH || word > MAX_HASH {
                continue;
            }

            let attr_hash = word;
            let noun_hash = cursor.next_word(self)?;
            let extra_info = cursor.next_word(self)?;

            entries.push(AttlibSyntaxEntry {
                attr_hash,
                noun_hash,
                extra_info,
            });
        }

        eprintln!("ATGTSX 加载完成，共 {} 条记录", entries.len());
        Ok(entries)
    }

    /// 构建 Noun -> 属性哈希列表映射
    pub fn build_noun_attr_mapping(&mut self) -> std::io::Result<NounAttrMapping> {
        let entries = self.load_atgtsx()?;
        let mut mapping: NounAttrMapping = HashMap::new();

        for entry in entries {
            mapping
                .entry(entry.noun_hash)
                .or_default()
                .push(entry.attr_hash);
        }

        eprintln!("构建映射完成，共 {} 个 Noun 类型", mapping.len());
        Ok(mapping)
    }

    /// 获取指定 Noun 的所有属性定义
    ///
    /// # Arguments
    /// * `noun_hash` - 类型的哈希值（可通过 encode_base27() 从名称计算）
    pub fn get_noun_attributes(
        &mut self,
        noun_hash: u32,
    ) -> std::io::Result<Vec<AttlibAttrDefinition>> {
        let mapping = self.build_noun_attr_mapping()?;

        let attr_hashes = mapping.get(&noun_hash).cloned().unwrap_or_default();
        let mut attrs = Vec::new();

        for hash in attr_hashes {
            if let Some(def) = self.attr_definitions.get(&hash) {
                attrs.push(def.clone());
            }
        }

        Ok(attrs)
    }

    /// 获取指定 Noun 的属性名称列表
    pub fn get_noun_attribute_names(&mut self, noun_name: &str) -> std::io::Result<Vec<String>> {
        let noun_hash = encode_base27(noun_name);
        let mapping = self.build_noun_attr_mapping()?;

        let attr_hashes = mapping.get(&noun_hash).cloned().unwrap_or_default();
        let names: Vec<String> = attr_hashes.iter().map(|h| decode_base27(*h)).collect();

        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_base27() {
        // 测试已知值
        assert_eq!(decode_base27(0xCA439), "ELBO");
        assert_eq!(decode_base27(639374), "NAME");
    }

    #[test]
    fn test_encode_base27() {
        // 测试编码
        assert_eq!(encode_base27("ELBO"), 0xCA439);
        assert_eq!(encode_base27("NAME"), 639374);
        // PIPE 的正确哈希值
        let pipe_hash = encode_base27("PIPE");
        assert!(pipe_hash >= MIN_HASH && pipe_hash <= MAX_HASH);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // 往返测试
        let names = ["ELBO", "PIPE", "NAME", "BORE", "TEMP"];
        for name in names {
            let hash = encode_base27(name);
            let decoded = decode_base27(hash);
            assert_eq!(decoded, name, "Roundtrip failed for {}", name);
        }
    }
}
