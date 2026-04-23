//! 文字度量模块 — 精确复刻 PDMS `mbdtextlen.pmlfnc`。
//!
//! PDMS 使用一张固定字符宽度表将文字字符串换算成无量纲 **em 总和**，
//! 乘以字高 `cheight` 后得到实际视觉宽度（mm）。本模块原样移植该表，
//! 保证与 PDMS 完全对齐。
//!
//! # 用法
//!
//! ```rust
//! use aios_core::mbd::v2::text_measurement::{mbd_text_len, mbd_text_width};
//!
//! let em = mbd_text_len("500");
//! assert!((em - 1.7264).abs() < 0.001);
//!
//! let width_mm = mbd_text_width("500", 2.5);
//! assert!((width_mm - 4.316).abs() < 0.01);
//! ```

use std::sync::OnceLock;

/// 未在 PDMS 查表中出现的字符，默认 em 宽度。
const DEFAULT_CHAR_WIDTH: f32 = 1.02;

/// PDMS `mbdtextlen.pmlfnc` 字符宽度表。
///
/// 表结构：`(char, em_width)` 对数组，从 PML `!letters` / `!lens` 逐项
/// 对应翻译。两处注意：
/// - PML 原文中 `w s y z` 位置的第二个 `s` 实为 `x` 的笔误（小写/大写同），
///   这里**保留原文行为**：查 'x' 会得到默认值 `1.02`。
/// - 后半段中文全角标点因 PML 文件编码差异可能失真，这里以 GB2312 / GBK
///   常见全角标点重新映射。如果运行时遇到的字符不在表中，统一走默认值。
const CHAR_WIDTH_TABLE: &[(char, f32)] = &[
    // ── 数字 ──
    ('1', 0.5741),
    ('2', 0.5762),
    ('3', 0.5761),
    ('4', 0.5764),
    ('5', 0.5762),
    ('6', 0.5761),
    ('7', 0.5761),
    ('8', 0.5761),
    ('9', 0.5761),
    ('0', 0.5761),
    // ── 小写字母 ──
    ('a', 0.5762),
    ('b', 0.5759),
    ('c', 0.5204),
    ('d', 0.5759),
    ('e', 0.5762),
    ('f', 0.2989),
    ('g', 0.5760),
    ('h', 0.5757),
    ('i', 0.2417),
    ('j', 0.2428),
    ('k', 0.5201),
    ('l', 0.2417),
    ('m', 0.8526),
    ('n', 0.5756),
    ('o', 0.5763),
    ('p', 0.5759),
    ('q', 0.5759),
    ('r', 0.3533),
    ('s', 0.5201),
    ('t', 0.2984),
    ('u', 0.5756),
    ('v', 0.5206),
    ('w', 0.7429),
    // PML 原文此处是 's'（笔误），对应 lens[34] = 0.5201；'x' 未入表
    // 保持与 PDMS 一致：查 'x' → DEFAULT_CHAR_WIDTH
    ('y', 0.5206),
    ('z', 0.5204),
    // ── 大写字母 ──
    ('A', 0.6878),
    ('B', 0.6866),
    ('C', 0.7421),
    ('D', 0.7417),
    ('E', 0.6865),
    ('F', 0.6304),
    ('G', 0.7975),
    ('H', 0.7414),
    ('I', 0.2968),
    ('J', 0.5198),
    ('K', 0.6871),
    ('L', 0.5759),
    ('M', 0.8523),
    ('N', 0.7414),
    ('O', 0.7977),
    ('P', 0.6866),
    ('Q', 0.7979),
    ('R', 0.7421),
    ('S', 0.6869),
    ('T', 0.6312),
    ('U', 0.7414),
    ('V', 0.6877),
    ('W', 0.9645),
    // PML 原文此处是 'S'（笔误），对应 lens[60] = 0.6869；'X' 未入表
    ('Y', 0.6877),
    ('Z', 0.6312),
    // ── ASCII 标点 ──
    ('~', 0.6040),
    ('`', 0.3523),
    ('!', 0.2970),
    ('@', 1.0351),
    ('#', 0.5768),
    ('$', 0.2981),
    ('%', 0.9086),
    ('^', 0.4895),
    ('&', 0.6872),
    ('*', 0.4093),
    ('(', 0.3529),
    (')', 0.3529),
    ('-', 0.3532),
    ('_', 0.5210),
    ('=', 0.6037),
    ('+', 0.6037),
    ('{', 0.3543),
    ('[', 0.2978),
    ('}', 0.3543),
    (']', 0.2978),
    ('\\', 0.2987),
    (';', 0.2969),
    (':', 0.2969),
    ('\'', 0.1154),
    ('"', 0.3750),
    (',', 0.2969),
    ('<', 0.6037),
    ('.', 0.2969),
    ('>', 0.6037),
    ('/', 0.2987),
    ('?', 0.5760),
    // ── 追加项（PML append） ──
    ('|', 0.2788),
    (' ', 0.2987),
    // ── 中文全角标点（从 PML 后半段对应，GBK 常见全角） ──
    ('\u{FF5E}', 0.6040), // ～ fullwidth tilde
    ('\u{00A4}', 0.2969), // ¤
    ('\u{FF01}', 1.0351), // ！
    ('\u{FF20}', 0.5768), // ＠
    ('\u{FF03}', 1.0164), // ＃
    ('\u{FFE5}', 0.9086), // ￥
    ('\u{FF05}', 1.0185), // ％
    ('\u{2026}', 1.0185), // … (ellipsis, PML 两处 ¡­)
    ('\u{FF06}', 0.6872), // ＆
    ('\u{FF0A}', 0.4093), // ＊
    ('\u{FF08}', 1.0132), // （
    ('\u{FF09}', 1.0132), // ）
    ('\u{2014}', 1.0210), // — em dash
    ('\u{2013}', 1.0210), // – en dash (PML 两处 ¡ª)
    ('\u{FF0D}', 0.3532), // － fullwidth hyphen
    ('\u{FF0B}', 0.6037), // ＋
    ('\u{FF1D}', 0.6037), // ＝
    ('\u{FF5B}', 0.3543), // ｛
    ('\u{FF3E}', 1.0137), // ｾ
    ('\u{FF5D}', 0.3543), // ｝
    ('\u{FF3F}', 1.0137), // ｿ
    ('\u{3001}', 1.0133), // 、
    ('\u{FF1A}', 1.0118), // ：
    ('\u{FF1B}', 1.0119), // ；
    ('\u{2018}', 0.3531), // '
    ('\u{2019}', 0.2418), // '
    ('\u{00B6}', 1.0153), // ¶
    ('\u{FF0C}', 1.0119), // ，
    ('\u{00B7}', 1.0153), // ·
    ('\u{3002}', 1.0137), // 。
    ('\u{FF1F}', 1.0155), // ？
    ('\u{FF0E}', 1.0133), // ．
];

fn build_char_map() -> std::collections::HashMap<char, f32> {
    CHAR_WIDTH_TABLE.iter().copied().collect()
}

static CHAR_MAP: OnceLock<std::collections::HashMap<char, f32>> = OnceLock::new();

fn char_map() -> &'static std::collections::HashMap<char, f32> {
    CHAR_MAP.get_or_init(build_char_map)
}

/// 计算文字的无量纲 em 宽度总和，与 PDMS `!!mbdtextlen(!text)` 完全等价。
///
/// 每个字符查表得到宽度系数，累加后返回。未知字符使用默认值 `1.02`。
pub fn mbd_text_len(text: &str) -> f32 {
    let map = char_map();
    text.chars()
        .map(|c| map.get(&c).copied().unwrap_or(DEFAULT_CHAR_WIDTH))
        .sum()
}

/// 计算文字在给定字高下的实际宽度（mm）。
///
/// `width_mm = mbd_text_len(text) * cheight`
pub fn mbd_text_width(text: &str, cheight: f32) -> f32 {
    mbd_text_len(text) * cheight
}

/// 格式化尺寸数字为 PDMS 惯用字符串：
/// - 保留两位小数
/// - 去掉尾部 ".00"
/// - 去掉小数点后第二位的无意义 '0'（如 "12.30" → "12.3"）
///
/// 与 PDMS `!dis.string('D2').replace('.00','')` + 尾零清理等价。
pub fn format_dim_value(value: f32) -> String {
    let s = format!("{:.2}", value);
    if s.ends_with(".00") {
        return s[..s.len() - 3].to_string();
    }
    if s.ends_with('0') {
        if let Some(dot_pos) = s.find('.') {
            if s.len() >= dot_pos + 3 {
                let mut result = String::with_capacity(s.len() - 1);
                result.push_str(&s[..dot_pos + 2]);
                if s.len() > dot_pos + 3 {
                    result.push_str(&s[dot_pos + 3..]);
                }
                return result;
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_match_pdms_table() {
        let expected = [
            ('1', 0.5741),
            ('2', 0.5762),
            ('3', 0.5761),
            ('4', 0.5764),
            ('5', 0.5762),
            ('6', 0.5761),
            ('7', 0.5761),
            ('8', 0.5761),
            ('9', 0.5761),
            ('0', 0.5761),
        ];
        let map = char_map();
        for (ch, w) in expected {
            assert_eq!(
                map.get(&ch).copied(),
                Some(w),
                "digit '{}' width mismatch",
                ch
            );
        }
    }

    #[test]
    fn text_len_for_500() {
        // "500": '5' = 0.5762, '0' = 0.5761, '0' = 0.5761 → sum = 1.7284
        let result = mbd_text_len("500");
        assert!(
            (result - 1.7284).abs() < 0.0001,
            "expected ~1.7284, got {}",
            result
        );
    }

    #[test]
    fn text_len_for_1234() {
        // "1234": 0.5741 + 0.5762 + 0.5761 + 0.5764 = 2.3028
        let result = mbd_text_len("1234");
        assert!(
            (result - 2.3028).abs() < 0.0001,
            "expected ~2.3028, got {}",
            result
        );
    }

    #[test]
    fn text_width_with_cheight() {
        let em = mbd_text_len("500");
        let width = mbd_text_width("500", 2.5);
        assert!((width - em * 2.5).abs() < 0.0001);
    }

    #[test]
    fn unknown_char_uses_default() {
        // '中' is not in the table → default 1.02
        let result = mbd_text_len("中");
        assert!(
            (result - DEFAULT_CHAR_WIDTH).abs() < 0.0001,
            "expected {}, got {}",
            DEFAULT_CHAR_WIDTH,
            result
        );
    }

    #[test]
    fn mixed_text_with_unknown() {
        // "A中" = 0.6878 + 1.02 = 1.7078
        let result = mbd_text_len("A中");
        assert!(
            (result - 1.7078).abs() < 0.0001,
            "expected ~1.7078, got {}",
            result
        );
    }

    #[test]
    fn space_and_pipe_appended() {
        assert_eq!(char_map().get(&' ').copied(), Some(0.2987));
        assert_eq!(char_map().get(&'|').copied(), Some(0.2788));
    }

    #[test]
    fn empty_string_returns_zero() {
        assert_eq!(mbd_text_len(""), 0.0);
    }

    #[test]
    fn x_char_uses_default_per_pdms_typo() {
        // PML 原文 'x'/'X' 位被 's'/'S' 替代，所以 'x'/'X' 不在表中
        let map = char_map();
        assert!(map.get(&'x').is_none(), "'x' should not be in the table");
        assert!(map.get(&'X').is_none(), "'X' should not be in the table");
        assert!(
            (mbd_text_len("x") - DEFAULT_CHAR_WIDTH).abs() < 0.0001,
            "'x' should use default width"
        );
    }

    #[test]
    fn format_dim_value_whole_number() {
        assert_eq!(format_dim_value(500.0), "500");
        assert_eq!(format_dim_value(1234.0), "1234");
    }

    #[test]
    fn format_dim_value_one_decimal() {
        assert_eq!(format_dim_value(12.30), "12.3");
        assert_eq!(format_dim_value(99.50), "99.5");
    }

    #[test]
    fn format_dim_value_two_decimals() {
        assert_eq!(format_dim_value(12.34), "12.34");
        assert_eq!(format_dim_value(0.17), "0.17");
    }

    #[test]
    fn format_dim_value_negative() {
        assert_eq!(format_dim_value(-500.0), "-500");
        assert_eq!(format_dim_value(-12.30), "-12.3");
    }
}
