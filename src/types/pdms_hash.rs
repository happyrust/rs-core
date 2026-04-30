#[inline]
pub fn convert_to_hash(bytes: &[u8]) -> i32 {
    i32::from_be_bytes(bytes.try_into().unwrap())
}

#[inline]
pub fn is_uda(hash: i32) -> bool {
    hash > 0x171FAD39
}

#[inline]
pub fn is_uda_name(name: &str) -> bool {
    name.starts_with(":")
}

#[inline]
pub fn get_uda_index(hash: u32) -> Option<u32> {
    if hash > 0x171FAD39 {
        Some((hash - 0x171FAD39) >> 24)
    } else {
        None
    }
}

#[inline]
pub fn db1_hash_i32(hash_str: &str) -> i32 {
    db1_hash(hash_str) as _
}

#[inline]
pub fn db1_hash(hash_str: &str) -> u32 {
    let chars = hash_str.as_bytes();
    if chars.is_empty() {
        return 0;
    }
    let mut val = 0i64;
    let mut i = (chars.len() - 1) as i32;
    while i >= 0 {
        val = val.overflowing_mul(27).0 + (chars[i as usize] as i64 - 64);
        i -= 1;
    }
    val.saturating_add_unsigned(0x81BF1) as u32
}

pub const fn db1_hash_const(hash_str: &str) -> u32 {
    let chars = hash_str.as_bytes();
    if chars.is_empty() {
        return 0;
    }
    let mut val = 0i64;
    let mut i = (chars.len() - 1) as i32;
    while i >= 0 {
        val = val.overflowing_mul(27).0 + (chars[i as usize] as i64 - 64);
        i -= 1;
    }
    val.saturating_add_unsigned(0x81BF1) as u32
}

#[inline]
pub fn db1_dehash(hash: u32) -> String {
    let mut result = String::new();
    if hash > 0x171FAD39 {
        let mut k = ((hash - 0x171FAD39) % 0x1000000) as i32;
        result.push(':');
        for _i in 0..6 {
            if k <= 0 {
                break;
            }
            result.push((k % 64 + 32) as u8 as char);
            k /= 64;
        }
    } else {
        if hash <= 0x81BF1 {
            return String::new();
        }
        let mut k = (hash - 0x81BF1) as i32;
        while k > 0 {
            result.push((k % 27 + 64) as u8 as char);
            k /= 27;
        }
    }
    result
}
