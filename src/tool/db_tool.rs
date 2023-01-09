use std::fs::File;
use std::io::Read;

use memchr::memmem::{find, find_iter};

use crate::pdms_types::PdmsDatabaseInfo;

/// 从bincode数据加载PdmsDatabaseInfo
pub fn read_attr_info_config_from_bin(config_path: &str) -> PdmsDatabaseInfo {
    let mut file = File::open(config_path).unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    bincode::deserialize(&attr_buf).unwrap()
}

/// 从json数据加载PdmsDatabaseInfo
pub fn read_attr_info_config_from_json(config_path: &str) -> PdmsDatabaseInfo {
    let mut file = File::open(config_path).unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    serde_json::from_slice(&attr_buf).unwrap()
}

#[inline]
pub fn convert_to_hash(bytes: &[u8]) -> u32 {
    i32::from_be_bytes(bytes.try_into().unwrap()).abs() as u32
}

#[inline]
pub fn is_uda(hash: u32) -> bool {
    hash > 0x171FAD39
}

#[inline]
pub fn db1_dehash(hash: u32) -> String {
    let mut result = String::new();
    if hash > 0x171FAD39 { // UDA的情况
        let mut v8 = ((hash - 0x171FAD39) % 0x1000000) as i32;
        result.push(':');
        for i in 0..6 {
            if v8 <= 0 {
                break;
            }
            result.push(((v8 & 0x3F) + 32) as u8 as char);
            v8 /= 64;
        }
    } else {
        if hash <= 0x81BF1 {
            return "".to_string();
        }
        let mut v6 = (hash - 0x81BF1) as i32;
        while v6 > 0 {
            result.push((v6 % 27 + 64) as u8 as char);
            v6 /= 27;
        }
    }
    result
}

//todo 处理出错的情况
#[inline]
pub const fn db1_hash(hash_str: &str) -> u32 {
    let mut chars = hash_str.as_bytes();
    if chars.len() < 1 {
        return 0;  //出错的暂时用0 表达
    }
    let mut val = 0i64;
    let mut i = (chars.len() - 1) as i32;
    while i >= 0 {
        val = val.overflowing_mul(27).0 + (chars[i as usize] as i64 - 64);
        i -= 1;
    }
    val.saturating_add_unsigned(0x81BF1) as u32
    // 0x81BF1 + val as u32
}

#[test]
fn db1_dehash_test() {
    let name = db1_dehash(0x95B0C);
    println!("name={:?}", name);

    let val = db1_hash("SCTN");
    println!("{:#4X}", val);
}


fn convert_to_le_i32(table: &[u8], dw_offset: usize) -> i32 {
    i32::from_le_bytes(table[dw_offset * 4..dw_offset * 4 + 4].try_into().unwrap())
}

fn get_mapped_value(table: &[u8], v: i32) -> i32 {
    let mut res = -1;
    if v < 0xFFFF {
        if v < 0 {
            res = -1;
        } else if v <= 127 {
            res = v;
        } else if v <= 255 {
            res = convert_to_le_i32(table, v as usize) as i32;
        } else {
            res = convert_to_le_i32(table, (v / 0x100) as usize);
        }
    }

    if res == -99999 || v < 0x40 || v == 0xFF {
        return -1;
    }

    if res < 0 {
        let c = res * -764i32;
        //println!("{:#4X}", c);
        let a = v & 0xFF;
        // //println!("{:#4X}", a);
        //println!("{}", (a + (c / 4) as i32 + 1) as usize);
        res = convert_to_le_i32(table, (a + (c / 4) as i32 + 1) as usize) as i32;  //从1开始
    }
    res
}

pub fn convert_to_u8_vec(v: i64) -> Vec<u8> {
    let mut res = vec![];
    if v < 0 {} else if v <= 127 {
        res.push(v as u8);
    } else if v <= 2047 {
        res.push((v / 0x40 + 0xC0) as u8);
        res.push((v % 0x40 + 0x80) as u8);
    } else if v <= 0xFFFF {
        res.push((v / 0x1000 + 0xE0) as u8);
        res.push(((v & 0x80000FFF) / 0x40 + 0x80) as u8);
        res.push((v % 0x40 + 0x80) as u8);
    } else if v > 0x10FFFF {
        //todo
    } else {
        res.push((v / 0x40000 + 240) as u8);
        res.push(((v & 0x8003FFFF) / 0x1000 + 0x80) as u8);
        res.push(((v & 0x80000FFF) / 0x40 + 0x80) as u8);
        res.push((v % 0x40 + 0x80) as u8);
    }
    return res;
}

///处理这种数据 [0x26, 0x7E, 0xxx, 0xxx, 0x20, 0x26]
pub fn decode_chi_chars(table: &[u8], data: &[u8]) -> String {
    if data.len() < 6 {
        return "unset".to_string();
    }
    let mut str_data = vec![];
    let mut d = &data[2..data.len() - 2];
    //println!("{:#4X?}", d);
    let mut i = 0;
    while i < d.len() - 1 { //todo 原来是 d.len() 加了个 -1 ，该方法可能需要调整
        let d0 = d[i] as u64;
        //println!("{:#4X?}", d0);
        let d1 = d[i + 1] as i32;
        //println!("{:#4X?}", d1);
        let val = (d0 << 8) as i32 + d1 + 0x8080;
        //println!("{:#4X?}", val);
        let code = get_mapped_value(table, val) as i64;
        //println!("Code: {:#4X?}", code);
        let chars = convert_to_u8_vec(code);
        //println!("Code: {:#4X?}", &chars);
        str_data.extend(chars);
        i += 2;
    }
    String::from_utf8_lossy(&str_data).to_string()
}

///返回（解密后的utf8字符串， 是否包含中文）
pub fn decode_chars_data(input: &[u8]) -> (String, bool) {
    let table_data = include_bytes!("../../encode_char_table.bin");
    let start_iter = find_iter(&input, &[0x26, 0x7E]);
    let mut contains_chi = false;
    let mut res = vec![];
    let mut prev_pos = 0;
    for p in start_iter {
        if input.len() > p && prev_pos <= p {
            res.extend_from_slice(&input[prev_pos..p]);
            if let Some(len) = find(&input[p..], &[0x20, 0x26]) {
                let decode_str = decode_chi_chars(table_data, &input[p..p + len + 2]);
                res.extend(decode_str.bytes());
                prev_pos = p + len + 2;
                contains_chi = true;
            } else {
                res.extend_from_slice(&input[p..]);
                break;
            }
        }
    }
    res.extend_from_slice(&input[prev_pos..]);

    (String::from_utf8_lossy(&res).to_string(), contains_chi)
}

#[test]
fn test_chinese_data() {
    // //println!("Hello, world!");
    //26 7E 37 27 43 45 20 26
    let test_code = vec![0x26, 0x7E, 0x37, 0x56, 0x56, 0x27, 0x32, 0x62, 0x4A, 0x54, 0x20, 0x26];
    let test_code = vec![0x26, 0x7E, 0x37, 0x27, 0x43, 0x45, 0x20, 0x26];
    let test_code = vec![0x26, 0x7E, 0x39, 0x5C, 0x20, 0x26];
    let test_code = vec![0x2F, 0x26, 0x7E, 0x39, 0x5C, 0x20, 0x26, 0x31, 0x32, 0x26, 0x7E, 0x35, 0x40, 0x20, 0x26];
    let test_code = vec![0x2F, 0x26, 0x7E, 0x39, 0x5C, 0x20, 0x26, 0x31, 0x32, 0x26, 0x7E, 0x35, 0x40, 0x20, 0x26, 0x74, 0x65, 0x73, 0x74];
    let test_code = vec![0x2F, 0x31, 0x30, 0x30, 0x2D, 0x42, 0x2D, 0x31];
    let test_code = vec![0x26, 0x7E, 0x4F, 0x7A, 0x20, 0x26, 0x7E, 0x26, 0x7E, 0x4F, 0x7A, 0x33, 0x5F, 0x34, 0x67, 0x20, 0x26];
    // let table_data = include_bytes!("../encode_char_table.bin");

    let name = decode_chars_data(&test_code);
    dbg!(name);
}




