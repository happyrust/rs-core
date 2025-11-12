///! 解析 attlib.dat 的 ATGTSX 表，提取 NOUN 类型层级关系
///!
///! 基于 IDA Pro 分析的 ATTLIB_Load_Syntax_ATGTSX 函数 (0x108533B4)
///!
///! ATGTSX 表结构：
///! - pack_code: 属性hash（可能包含 NOUN 类型信息）
///! - index_ptr: 索引指针
///! - third_value: 第三个值（用途待确定）
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const PAGE_SIZE: usize = 2048;
const WORDS_PER_PAGE: usize = 512;
const PAGE_SWITCH_MARK: u32 = 0x00000000;
const SEGMENT_END_MARK: u32 = 0xFFFFFFFF;
const DATA_REGION_START: u64 = 0x1000;
const SEGMENT_POINTERS_OFFSET: u64 = 0x0800;

// OWNER 属性的 hash 值
const OWNER_HASH: u32 = 0x88CFEC; // db1_hash("OWNER") = 8966124

#[derive(Debug, Clone)]
struct AtgtSxRecord {
    pack_code: u32,
    index_ptr: u32,
    third_value: u32,
}

struct AttlibParser {
    file: File,
    segment_pointers: [u32; 8],
    page_cache: HashMap<u32, Vec<u32>>,
}

impl AttlibParser {
    fn new(file_path: &str) -> std::io::Result<Self> {
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
            segment_pointers,
            page_cache: HashMap::new(),
        })
    }

    fn read_page(&mut self, page_num: u32) -> std::io::Result<Vec<u32>> {
        if let Some(cached) = self.page_cache.get(&page_num) {
            return Ok(cached.clone());
        }

        let file_offset = DATA_REGION_START + (page_num as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(file_offset))?;

        let mut page_data = vec![0u8; PAGE_SIZE];
        self.file.read_exact(&mut page_data)?;

        let mut words = Vec::with_capacity(WORDS_PER_PAGE);
        for i in 0..WORDS_PER_PAGE {
            let bytes = &page_data[i * 4..(i + 1) * 4];
            let word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            words.push(word);
        }

        self.page_cache.insert(page_num, words.clone());
        Ok(words)
    }

    /// 加载 ATGTSX 段
    /// 根据 IDA 分析，ATGTSX 在段指针 [3] 位置
    fn load_atgtsx(&mut self) -> std::io::Result<Vec<AtgtSxRecord>> {
        let start_page = self.segment_pointers[3];
        eprintln!("\n加载 ATGTSX 段");
        eprintln!("  起始页号: {} (段指针 [3])", start_page);

        let mut records = Vec::new();
        let mut page_num = start_page;
        let mut word_idx = 0;

        loop {
            let words = self.read_page(page_num)?;

            while word_idx < WORDS_PER_PAGE {
                let word = words[word_idx];
                word_idx += 1;

                // 页切换标记
                if word == PAGE_SWITCH_MARK {
                    page_num += 1;
                    word_idx = 0;
                    break;
                }

                // 段结束标记
                if word == SEGMENT_END_MARK {
                    eprintln!("  ATGTSX 加载完成，共 {} 条记录", records.len());
                    return Ok(records);
                }

                // 读取三元组：pack_code, index_ptr, third_value
                let pack_code = word;

                if word_idx >= WORDS_PER_PAGE {
                    page_num += 1;
                    word_idx = 0;
                    continue;
                }
                let index_ptr = words[word_idx];
                word_idx += 1;

                if word_idx >= WORDS_PER_PAGE {
                    page_num += 1;
                    word_idx = 0;
                    continue;
                }
                let third_value = words[word_idx];
                word_idx += 1;

                records.push(AtgtSxRecord {
                    pack_code,
                    index_ptr,
                    third_value,
                });

                if records.len() < 10 {
                    eprintln!(
                        "    [{}] pack_code=0x{:08X}, index_ptr={}, third_value={}",
                        records.len() - 1,
                        pack_code,
                        index_ptr,
                        third_value
                    );
                }
            }
        }
    }
}

/// 27 进制解码（基于 IDA 分析的 sub_10853FC8 函数）
fn decode_pack_code(pack_code: u32) -> String {
    let mut result = Vec::new();
    let mut value = pack_code;

    for _ in 0..5 {
        let remainder = value % 27;
        value /= 27;

        if remainder > 0 {
            result.push((remainder + 64) as u8 as char);
        } else {
            break;
        }
    }

    result.reverse();
    result.into_iter().collect()
}

/// db1_hash 函数
fn db1_hash(s: &str) -> u32 {
    let s = s.to_uppercase();
    let mut h = 0u32;
    for c in s.chars() {
        h = h * 27 + (c as u32 - 'A' as u32 + 1);
    }
    h + 0x81BF1
}

/// db1_dehash 函数
fn db1_dehash(hash: u32) -> String {
    if hash < 0x81BF1 {
        return format!("INVALID_HASH_{}", hash);
    }

    let mut value = hash - 0x81BF1;
    let mut result = Vec::new();

    while value > 0 {
        let remainder = value % 27;
        value /= 27;

        if remainder > 0 {
            result.push(((remainder - 1) as u8 + b'A') as char);
        }
    }

    result.reverse();
    result.into_iter().collect()
}

fn main() -> std::io::Result<()> {
    eprintln!("=== ATGTSX 表解析器 ===\n");

    // 1. 加载 attlib.dat
    let mut parser = AttlibParser::new("data/attlib.dat")?;

    // 2. 加载 ATGTSX 表
    let records = parser.load_atgtsx()?;

    eprintln!("\n=== 分析 ATGTSX 记录 ===");
    eprintln!("总记录数: {}", records.len());

    // 3. 分析 pack_code 的编码方式
    eprintln!("\n=== 分析 pack_code 编码 ===");
    eprintln!("OWNER hash: 0x{:08X} ({})", OWNER_HASH, OWNER_HASH);

    // 尝试查找包含 "OWNER" 字符串的记录
    let owner_decoded_records: Vec<_> = records
        .iter()
        .filter(|r| {
            let decoded = decode_pack_code(r.pack_code);
            let dehashed = db1_dehash(r.pack_code);
            decoded.contains("OWNER") || dehashed.contains("OWNER")
        })
        .collect();

    eprintln!("找到 {} 条包含 'OWNER' 的记录", owner_decoded_records.len());

    for (i, record) in owner_decoded_records.iter().take(20).enumerate() {
        let decoded = decode_pack_code(record.pack_code);
        let dehashed = db1_dehash(record.pack_code);
        eprintln!(
            "  [{}] pack_code=0x{:08X}, index_ptr={}, third_value={}",
            i, record.pack_code, record.index_ptr, record.third_value
        );
        eprintln!("      decoded: '{}', dehashed: '{}'", decoded, dehashed);
    }

    // 分析 pack_code 的位模式
    eprintln!("\n=== pack_code 位模式分析 ===");
    eprintln!("前 20 条记录的 pack_code 分解:");
    for (i, record) in records.iter().take(20).enumerate() {
        let high_byte = (record.pack_code >> 24) & 0xFF;
        let mid_bytes = (record.pack_code >> 8) & 0xFFFF;
        let low_byte = record.pack_code & 0xFF;
        eprintln!(
            "  [{}] 0x{:08X} = [0x{:02X}][0x{:04X}][0x{:02X}] decoded='{}' dehashed='{}'",
            i,
            record.pack_code,
            high_byte,
            mid_bytes,
            low_byte,
            decode_pack_code(record.pack_code),
            db1_dehash(record.pack_code)
        );
    }

    // 4. 统计 pack_code 的分布
    eprintln!("\n=== pack_code 统计 ===");
    let mut pack_code_freq: HashMap<u32, usize> = HashMap::new();
    for record in &records {
        *pack_code_freq.entry(record.pack_code).or_insert(0) += 1;
    }

    let mut freq_vec: Vec<_> = pack_code_freq.iter().collect();
    freq_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    eprintln!("前 20 个最常见的 pack_code:");
    for (pack_code, count) in freq_vec.iter().take(20) {
        let decoded = decode_pack_code(**pack_code);
        let dehashed = db1_dehash(**pack_code);
        eprintln!(
            "  0x{:08X}: {} 次 (decoded='{}', dehashed='{}')",
            pack_code, count, decoded, dehashed
        );
    }

    // 5. 深入分析 OWNER 记录
    eprintln!("\n=== 深入分析 OWNER 记录 ===");
    if let Some(owner_record) = owner_decoded_records.first() {
        eprintln!("OWNER 记录:");
        eprintln!(
            "  pack_code: 0x{:08X} ('{}')",
            owner_record.pack_code,
            decode_pack_code(owner_record.pack_code)
        );
        eprintln!("  index_ptr: {}", owner_record.index_ptr);
        eprintln!("  third_value: {}", owner_record.third_value);

        // 尝试读取 index_ptr 指向的数据
        // 根据 ATGTDF 的经验，index_ptr 可能指向另一个段的记录
        eprintln!("\n尝试解析 index_ptr 指向的数据...");

        // 段 [6] 和 [7] 可能包含相关数据
        for seg_idx in [6, 7] {
            eprintln!(
                "\n检查段 [{}] (页号 {}):",
                seg_idx, parser.segment_pointers[seg_idx]
            );
            let page = parser.read_page(parser.segment_pointers[seg_idx])?;

            // 尝试在该页中查找 index_ptr 附近的数据
            let idx = owner_record.index_ptr as usize;
            if idx < page.len() {
                eprintln!("  page[{}] = 0x{:08X}", idx, page[idx]);
                if idx > 0 {
                    eprintln!("  page[{}] = 0x{:08X}", idx - 1, page[idx - 1]);
                }
                if idx + 1 < page.len() {
                    eprintln!("  page[{}] = 0x{:08X}", idx + 1, page[idx + 1]);
                }
            }
        }
    }

    // 6. 分析所有 NOUN 相关的记录
    eprintln!("\n=== 查找所有 NOUN 类型相关记录 ===");
    let known_nouns = vec![
        "WORL", "SITE", "ZONE", "EQUI", "PIPE", "BRAN", "ELBO", "TEE", "FLAN",
    ];

    for noun in &known_nouns {
        let noun_records: Vec<_> = records
            .iter()
            .filter(|r| {
                let decoded = decode_pack_code(r.pack_code);
                decoded == *noun
            })
            .collect();

        if !noun_records.is_empty() {
            eprintln!("\n{}: 找到 {} 条记录", noun, noun_records.len());
            for (i, record) in noun_records.iter().take(3).enumerate() {
                eprintln!(
                    "  [{}] pack_code=0x{:08X}, index_ptr={}, third_value={}",
                    i, record.pack_code, record.index_ptr, record.third_value
                );
            }
        }
    }

    eprintln!("\n=== 完成分析 ===");

    Ok(())
}
