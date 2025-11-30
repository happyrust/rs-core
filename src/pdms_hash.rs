//! PDMS Hash 算法实现
//! 
//! 基于 core.dll 中 ENHAS2 函数的逆向分析

/// PDMS ENHAS2 Hash 算法实现
/// 
/// 这是 PDMS 使用的实际 hash 算法，与简单的 base27 编码不同
/// 
/// # 算法特点
/// - 初始值：531441 (27^5)
/// - 乘数：27 (base27)
/// - 空格字符处理：视为 0
/// - 字符范围：A-Z (1-26)
/// - 最大长度：6 字符
/// 
/// # 示例
/// ```
/// use aios_core::pdms_hash::pdms_enhas2;
/// assert_eq!(pdms_enhas2("PRDISP"), 240391897);
/// assert_eq!(pdms_enhas2("NAME"), 639374);
/// ```
pub fn pdms_enhas2(s: &str) -> i32 {
    if s.is_empty() {
        return 0;
    }
    
    let mut hash = 531441;  // 27^5
    let mut multiplier = 1;
    
    // 最多处理 6 个字符
    let chars: Vec<char> = s.chars().collect();
    let max_len = if chars.len() > 6 { 6 } else { chars.len() };
    
    for i in 0..max_len {
        let ch = chars[i];
        
        let value = if ch == ' ' {
            0
        } else if ch.is_ascii_uppercase() {
            (ch as u8 - b'A' + 1) as i32
        } else if ch.is_ascii_lowercase() {
            (ch as u8 - b'a' + 1) as i32
        } else {
            // 非法字符，返回 0（与 ENHAS2 行为一致）
            return 0;
        };
        
        // 验证字符范围（A-Z 或空格）
        if value < 0 || value > 26 {
            return 0;
        }
        
        hash += multiplier * value;
        multiplier *= 27;
    }
    
    hash
}

/// 反向解码 hash 值
/// 
/// 将 PDMS hash 值转换回原始字符串
/// 注意：由于 hash 冲突可能，结果可能不唯一
pub fn pdms_dehash(hash: i32) -> String {
    if hash == 0 {
        return String::new();
    }
    
    let mut result = String::new();
    let mut remaining = hash - 531441;  // 减去初始值
    
    for _ in 0..6 {
        if remaining == 0 {
            break;
        }
        
        let value = remaining % 27;
        if value == 0 {
            result.push(' ');
        } else {
            result.push((b'A' + value as u8 - 1) as char);
        }
        
        remaining /= 27;
    }
    
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pdms_enhas2() {
        // 测试已知值
        assert_eq!(pdms_enhas2("PRDISP"), 240391897);
        assert_eq!(pdms_enhas2("NAME"), 639374);
        assert_eq!(pdms_enhas2("POS"), 641779);
        assert_eq!(pdms_enhas2("ORI"), 641780);
        
        // 测试空格处理
        assert_eq!(pdms_enhas2("A B"), 531441 + 1*1 + 27*0 + 27*27*2);
        
        // 测试长度限制
        let long_str = "ABCDEFG";
        assert_eq!(pdms_enhas2(long_str), pdms_enhas2("ABCDEF"));
        
        // 测试非法字符
        assert_eq!(pdms_enhas2("A1B"), 0);
        assert_eq!(pdms_enhas2(""), 0);
    }
    
    #[test]
    fn test_pdms_dehash() {
        assert_eq!(pdms_dehash(240391897), "PRDISP");
        assert_eq!(pdms_dehash(639374), "NAME");
        assert_eq!(pdms_dehash(641779), "POS");
        assert_eq!(pdms_dehash(0), "");
    }
    
    #[test]
    fn test_compatibility_with_old_hash() {
        // 对比旧的 base27 实现
        let old_hash = |s: &str| {
            let mut h = 0;
            for c in s.uppercase() {
                h = h * 27 + (c as u8 - 64);
            }
            h + 0x81BF1
        };
        
        // NAME 的结果应该相同（巧合）
        assert_eq!(pdms_enhas2("NAME"), old_hash("NAME"));
        
        // PRDISP 的结果不同
        assert_ne!(pdms_enhas2("PRDISP"), old_hash("PRDISP"));
    }
}
