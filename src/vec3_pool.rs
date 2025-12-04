//! Vec3 向量池模块
//!
//! 提供向量去重存储功能，将常见的方向向量映射为紧凑的 ID，
//! 减少数据库存储空间和网络传输开销。
//!
//! # 设计原理
//!
//! - 常见方向向量（如坐标轴方向、45°对角线等）使用预定义的 u8 ID (1-255)
//! - 非常见向量使用量化后的 u64 hash
//! - 量化精度为 0.001，足够工程应用
//!
//! # 使用示例
//!
//! ```rust
//! use aios_core::vec3_pool::{encode_direction, decode_direction, Vec3Id};
//! use glam::Vec3;
//!
//! // 编码
//! let dir = Vec3::new(1.0, 0.0, 0.0);
//! let id = encode_direction(dir);
//! assert!(matches!(id, Vec3Id::Common(1))); // 正X轴是预定义ID 1
//!
//! // 解码
//! let decoded = decode_direction(&id);
//! assert!((decoded - dir).length() < 0.01);
//! ```

use glam::Vec3;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::FRAC_1_SQRT_2; // 0.707...

/// 量化精度：乘数
const QUANTIZE_SCALE: f32 = 1000.0;

/// 量化后的整数元组类型
pub type QuantizedVec3 = (i32, i32, i32);

/// Vec3 的紧凑 ID 表示
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Vec3Id {
    /// 预定义的常见方向 (1-255)
    Common(u8),
    /// 非常见方向，使用量化 hash
    Hashed(u64),
    /// 零向量
    Zero,
}

impl Vec3Id {
    /// 转换为可存储的整数值
    /// - Common(n): 直接返回 n as i64
    /// - Hashed(h): 返回 h as i64 (负数表示 hashed)
    /// - Zero: 返回 0
    pub fn to_storage_value(&self) -> i64 {
        match self {
            Vec3Id::Common(n) => *n as i64,
            Vec3Id::Hashed(h) => -(*h as i64), // 负数表示 hashed
            Vec3Id::Zero => 0,
        }
    }

    /// 从存储值恢复
    pub fn from_storage_value(v: i64) -> Self {
        if v == 0 {
            Vec3Id::Zero
        } else if v > 0 && v <= 255 {
            Vec3Id::Common(v as u8)
        } else {
            Vec3Id::Hashed((-v) as u64)
        }
    }
}

// ============================================================================
// 预定义常见方向向量
// ============================================================================

/// 预定义方向表：QuantizedVec3 -> ID
static DIRECTION_TO_ID: Lazy<HashMap<QuantizedVec3, u8>> = Lazy::new(|| {
    let mut map = HashMap::new();
    let mut id: u8 = 1;

    // 辅助宏：插入向量及其量化表示
    macro_rules! insert_dir {
        ($v:expr) => {{
            let q = quantize_vec3($v);
            if !map.contains_key(&q) {
                map.insert(q, id);
                id += 1;
            }
        }};
    }

    // === 1. 主轴方向 (6个) ===
    insert_dir!(Vec3::X);           // (1, 0, 0)
    insert_dir!(Vec3::NEG_X);       // (-1, 0, 0)
    insert_dir!(Vec3::Y);           // (0, 1, 0)
    insert_dir!(Vec3::NEG_Y);       // (0, -1, 0)
    insert_dir!(Vec3::Z);           // (0, 0, 1)
    insert_dir!(Vec3::NEG_Z);       // (0, 0, -1)

    // === 2. 面对角线 45° (12个) ===
    let d45 = FRAC_1_SQRT_2; // 0.7071...
    // XY 平面
    insert_dir!(Vec3::new(d45, d45, 0.0));
    insert_dir!(Vec3::new(d45, -d45, 0.0));
    insert_dir!(Vec3::new(-d45, d45, 0.0));
    insert_dir!(Vec3::new(-d45, -d45, 0.0));
    // XZ 平面
    insert_dir!(Vec3::new(d45, 0.0, d45));
    insert_dir!(Vec3::new(d45, 0.0, -d45));
    insert_dir!(Vec3::new(-d45, 0.0, d45));
    insert_dir!(Vec3::new(-d45, 0.0, -d45));
    // YZ 平面
    insert_dir!(Vec3::new(0.0, d45, d45));
    insert_dir!(Vec3::new(0.0, d45, -d45));
    insert_dir!(Vec3::new(0.0, -d45, d45));
    insert_dir!(Vec3::new(0.0, -d45, -d45));

    // === 3. 体对角线 (8个) ===
    let d = 1.0 / 3.0_f32.sqrt(); // 0.577...
    insert_dir!(Vec3::new(d, d, d));
    insert_dir!(Vec3::new(d, d, -d));
    insert_dir!(Vec3::new(d, -d, d));
    insert_dir!(Vec3::new(d, -d, -d));
    insert_dir!(Vec3::new(-d, d, d));
    insert_dir!(Vec3::new(-d, d, -d));
    insert_dir!(Vec3::new(-d, -d, d));
    insert_dir!(Vec3::new(-d, -d, -d));

    // === 4. 常见角度：15° 增量 (XY, XZ, YZ 平面) ===
    // 角度值: 0, 15, 30, 45, 60, 75, 90, 105, 120, 135, 150, 165, 180...
    let angles_deg: [f32; 24] = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0, 150.0, 165.0, 180.0,
                      195.0, 210.0, 225.0, 240.0, 255.0, 270.0, 285.0, 300.0, 315.0, 330.0, 345.0];
    
    for &deg in &angles_deg {
        let rad = deg.to_radians();
        let c = rad.cos();
        let s = rad.sin();
        
        // XY 平面旋转
        if c.abs() > 0.001 || s.abs() > 0.001 {
            insert_dir!(Vec3::new(c, s, 0.0).normalize());
        }
        // XZ 平面旋转
        if c.abs() > 0.001 || s.abs() > 0.001 {
            insert_dir!(Vec3::new(c, 0.0, s).normalize());
        }
        // YZ 平面旋转
        if c.abs() > 0.001 || s.abs() > 0.001 {
            insert_dir!(Vec3::new(0.0, c, s).normalize());
        }
    }

    // === 5. 30°/60° 倾斜组合 ===
    let sin30 = 0.5_f32;
    let cos30 = 0.866_f32;
    let sin60 = cos30;
    let cos60 = sin30;

    // 30° 倾斜
    for &(c, s) in &[(cos30, sin30), (sin30, cos30)] {
        // 各种组合
        insert_dir!(Vec3::new(c, s, 0.0).normalize());
        insert_dir!(Vec3::new(c, -s, 0.0).normalize());
        insert_dir!(Vec3::new(-c, s, 0.0).normalize());
        insert_dir!(Vec3::new(-c, -s, 0.0).normalize());
        insert_dir!(Vec3::new(c, 0.0, s).normalize());
        insert_dir!(Vec3::new(c, 0.0, -s).normalize());
        insert_dir!(Vec3::new(-c, 0.0, s).normalize());
        insert_dir!(Vec3::new(-c, 0.0, -s).normalize());
        insert_dir!(Vec3::new(0.0, c, s).normalize());
        insert_dir!(Vec3::new(0.0, c, -s).normalize());
        insert_dir!(Vec3::new(0.0, -c, s).normalize());
        insert_dir!(Vec3::new(0.0, -c, -s).normalize());
    }

    // === 6. 22.5° 角度（管道弯头常用） ===
    let sin22 = 22.5_f32.to_radians().sin();
    let cos22 = 22.5_f32.to_radians().cos();
    insert_dir!(Vec3::new(cos22, sin22, 0.0).normalize());
    insert_dir!(Vec3::new(cos22, -sin22, 0.0).normalize());
    insert_dir!(Vec3::new(-cos22, sin22, 0.0).normalize());
    insert_dir!(Vec3::new(-cos22, -sin22, 0.0).normalize());
    insert_dir!(Vec3::new(sin22, cos22, 0.0).normalize());
    insert_dir!(Vec3::new(-sin22, cos22, 0.0).normalize());
    insert_dir!(Vec3::new(sin22, -cos22, 0.0).normalize());
    insert_dir!(Vec3::new(-sin22, -cos22, 0.0).normalize());

    println!("Vec3Pool: 预定义了 {} 个常见方向向量", map.len());
    map
});

/// 预定义方向表：ID -> Vec3
static ID_TO_DIRECTION: Lazy<HashMap<u8, Vec3>> = Lazy::new(|| {
    DIRECTION_TO_ID
        .iter()
        .map(|(&q, &id)| (id, dequantize_vec3(q)))
        .collect()
});

// ============================================================================
// 量化/反量化函数
// ============================================================================

/// 将 Vec3 量化为整数元组
#[inline]
pub fn quantize_vec3(v: Vec3) -> QuantizedVec3 {
    (
        (v.x * QUANTIZE_SCALE).round() as i32,
        (v.y * QUANTIZE_SCALE).round() as i32,
        (v.z * QUANTIZE_SCALE).round() as i32,
    )
}

/// 将量化后的整数元组还原为 Vec3
#[inline]
pub fn dequantize_vec3(q: QuantizedVec3) -> Vec3 {
    Vec3::new(
        q.0 as f32 / QUANTIZE_SCALE,
        q.1 as f32 / QUANTIZE_SCALE,
        q.2 as f32 / QUANTIZE_SCALE,
    )
}

/// 计算量化向量的 hash
#[inline]
fn hash_quantized(q: QuantizedVec3) -> u64 {
    // FNV-1a 风格的简单 hash
    let mut h: u64 = 0xcbf29ce484222325;
    h = h.wrapping_mul(0x100000001b3) ^ (q.0 as u64);
    h = h.wrapping_mul(0x100000001b3) ^ (q.1 as u64);
    h = h.wrapping_mul(0x100000001b3) ^ (q.2 as u64);
    h
}

// ============================================================================
// 公开 API：编码/解码
// ============================================================================

/// 编码方向向量为紧凑 ID
///
/// 自动归一化输入向量，然后查找预定义表或生成 hash
pub fn encode_direction(dir: Vec3) -> Vec3Id {
    // 处理零向量
    let len = dir.length();
    if len < 0.0001 {
        return Vec3Id::Zero;
    }

    // 归一化
    let normalized = dir / len;
    let q = quantize_vec3(normalized);

    // 查找预定义表
    if let Some(&id) = DIRECTION_TO_ID.get(&q) {
        return Vec3Id::Common(id);
    }

    // 非常见方向，使用 hash
    Vec3Id::Hashed(hash_quantized(q))
}

/// 解码紧凑 ID 为方向向量
///
/// 对于 Common ID 返回预定义向量，对于 Hashed 需要额外存储的原始值
pub fn decode_direction(id: &Vec3Id) -> Vec3 {
    match id {
        Vec3Id::Zero => Vec3::ZERO,
        Vec3Id::Common(n) => ID_TO_DIRECTION.get(n).copied().unwrap_or(Vec3::ZERO),
        Vec3Id::Hashed(_) => {
            // Hashed 类型需要从外部存储获取原始值
            // 这里返回零向量，调用者需要处理这种情况
            Vec3::ZERO
        }
    }
}

/// 编码位置向量（不归一化）
///
/// 返回量化后的 hash，用于去重存储
pub fn encode_position(pos: Vec3) -> u64 {
    let q = quantize_vec3(pos);
    hash_quantized(q)
}

/// 检查方向是否为预定义的常见方向
#[inline]
pub fn is_common_direction(dir: Vec3) -> bool {
    matches!(encode_direction(dir), Vec3Id::Common(_))
}

/// 获取预定义方向的数量
pub fn common_direction_count() -> usize {
    DIRECTION_TO_ID.len()
}

// ============================================================================
// 带存储的完整编解码（用于非常见向量）
// ============================================================================

/// 编码结果，包含 ID 和可能需要存储的原始值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedVec3 {
    pub id: Vec3Id,
    /// 非常见向量需要存储原始值
    pub raw: Option<[f32; 3]>,
}

impl EncodedVec3 {
    /// 从 Vec3 编码
    pub fn encode(v: Vec3) -> Self {
        let id = encode_direction(v);
        let raw = match id {
            Vec3Id::Hashed(_) => Some([v.x, v.y, v.z]),
            _ => None,
        };
        Self { id, raw }
    }

    /// 解码为 Vec3
    pub fn decode(&self) -> Vec3 {
        match self.id {
            Vec3Id::Zero => Vec3::ZERO,
            Vec3Id::Common(n) => ID_TO_DIRECTION.get(&n).copied().unwrap_or(Vec3::ZERO),
            Vec3Id::Hashed(_) => {
                self.raw
                    .map(|r| Vec3::new(r[0], r[1], r[2]))
                    .unwrap_or(Vec3::ZERO)
            }
        }
    }
}

// ============================================================================
// CateAxisParam 压缩存储
// ============================================================================

use crate::parsed_data::CateAxisParam;
use crate::shape::pdms_shape::RsVec3;
use crate::RefnoEnum;

/// 压缩版的 CateAxisParam，用于数据库存储
/// 
/// 优化点：
/// - dir/ref_dir 使用 Vec3Id 压缩（常见方向只需 1 字节）
/// - 位置为 ZERO 时省略 p 字段
/// - 省略默认值字段（pwidth=0, pheight=0 时不存储）
/// - refno 可选存储（多数场景不需要）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CateAxisParamCompact {
    /// 点编号（必需）
    pub n: i32,
    /// 位置坐标（ZERO 时省略）
    #[serde(skip_serializing_if = "is_zero_position")]
    pub p: Option<[f32; 3]>,
    /// 方向（压缩存储）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<EncodedVec3>,
    /// 方向标志（非1.0时存储）
    #[serde(skip_serializing_if = "is_default_dir_flag")]
    pub df: Option<f32>,
    /// 参考方向（压缩存储）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rd: Option<EncodedVec3>,
    /// 口径（非0时存储）
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub b: Option<f32>,
    /// 连接类型（非空时存储）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
    /// 宽度（非0时存储）
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub w: Option<f32>,
    /// 高度（非0时存储）
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub h: Option<f32>,
    /// 参考号（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<String>,
}

fn is_default_dir_flag(v: &Option<f32>) -> bool {
    v.map_or(true, |f| (f - 1.0).abs() < 0.001)
}

fn is_zero_f32(v: &Option<f32>) -> bool {
    v.map_or(true, |f| f.abs() < 0.001)
}

/// 检测位置是否为 ZERO（用于序列化时省略）
fn is_zero_position(v: &Option<[f32; 3]>) -> bool {
    v.map_or(true, |p| p[0].abs() < 0.001 && p[1].abs() < 0.001 && p[2].abs() < 0.001)
}

impl CateAxisParamCompact {
    /// 从 CateAxisParam 压缩
    pub fn from_full(param: &CateAxisParam, include_refno: bool) -> Self {
        // ZERO 位置设为 None，序列化时省略
        let pos = [param.pt.0.x, param.pt.0.y, param.pt.0.z];
        let p = if pos[0].abs() < 0.001 && pos[1].abs() < 0.001 && pos[2].abs() < 0.001 {
            None
        } else {
            Some(pos)
        };
        Self {
            n: param.number,
            p,
            d: param.dir.as_ref().map(|v| EncodedVec3::encode(v.0)),
            df: if (param.dir_flag - 1.0).abs() > 0.001 {
                Some(param.dir_flag)
            } else {
                None
            },
            rd: param.ref_dir.as_ref().map(|v| EncodedVec3::encode(v.0)),
            b: if param.pbore.abs() > 0.001 {
                Some(param.pbore)
            } else {
                None
            },
            c: if !param.pconnect.is_empty() && param.pconnect != "0" {
                Some(param.pconnect.clone())
            } else {
                None
            },
            w: if param.pwidth.abs() > 0.001 {
                Some(param.pwidth)
            } else {
                None
            },
            h: if param.pheight.abs() > 0.001 {
                Some(param.pheight)
            } else {
                None
            },
            r: if include_refno {
                Some(param.refno.to_string())
            } else {
                None
            },
        }
    }

    /// 还原为 CateAxisParam
    pub fn to_full(&self) -> CateAxisParam {
        // None 视为 ZERO
        let pt = self.p.map_or(Vec3::ZERO, |p| Vec3::new(p[0], p[1], p[2]));
        CateAxisParam {
            refno: self.r.as_ref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            number: self.n,
            pt: RsVec3(pt),
            dir: self.d.as_ref().map(|e| RsVec3(e.decode())),
            dir_flag: self.df.unwrap_or(1.0),
            ref_dir: self.rd.as_ref().map(|e| RsVec3(e.decode())),
            pbore: self.b.unwrap_or(0.0),
            pwidth: self.w.unwrap_or(0.0),
            pheight: self.h.unwrap_or(0.0),
            pconnect: self.c.clone().unwrap_or_default(),
        }
    }
}

/// 批量压缩 CateAxisParam 列表
pub fn compress_ptset(params: &[CateAxisParam], include_refno: bool) -> Vec<CateAxisParamCompact> {
    params.iter().map(|p| CateAxisParamCompact::from_full(p, include_refno)).collect()
}

/// 批量还原 CateAxisParam 列表
pub fn decompress_ptset(compacts: &[CateAxisParamCompact]) -> Vec<CateAxisParam> {
    compacts.iter().map(|c| c.to_full()).collect()
}

/// 计算压缩后的预估字节数
pub fn estimate_compressed_size(params: &[CateAxisParam]) -> usize {
    let mut size = 0;
    for p in params {
        // 基础字段: n(4) = 4 bytes
        size += 4;
        // 位置：ZERO 时省略（0 bytes），否则 12 bytes
        let pos = p.pt.0;
        if pos.x.abs() >= 0.001 || pos.y.abs() >= 0.001 || pos.z.abs() >= 0.001 {
            size += 12;
        }
        // 可选字段
        if let Some(dir) = p.dir.as_ref() {
            size += if is_common_direction(dir.0) { 1 } else { 13 };
        }
        if (p.dir_flag - 1.0).abs() > 0.001 { size += 4; }
        if let Some(ref_dir) = p.ref_dir.as_ref() {
            size += if is_common_direction(ref_dir.0) { 1 } else { 13 };
        }
        if p.pbore.abs() > 0.001 { size += 4; }
        if !p.pconnect.is_empty() && p.pconnect != "0" { size += p.pconnect.len() + 2; }
        if p.pwidth.abs() > 0.001 { size += 4; }
        if p.pheight.abs() > 0.001 { size += 4; }
    }
    size
}

/// 计算原始未压缩的预估字节数
pub fn estimate_original_size(params: &[CateAxisParam]) -> usize {
    // 每个 CateAxisParam 约 120-150 bytes (JSON格式)
    params.len() * 130
}

/// 从 JSON Value 解析 ptset（自动检测格式并解压）
/// 
/// 支持两种格式：
/// 1. 压缩格式：`[{n: 1, p: [...], d: {...}}, ...]`
/// 2. 原始格式：`[{number: 1, pt: [...], dir: [...], ...}, ...]`
pub fn parse_ptset_auto(value: &serde_json::Value) -> Option<Vec<CateAxisParam>> {
    let arr = value.as_array()?;
    if arr.is_empty() {
        return Some(Vec::new());
    }
    
    // 检测格式：压缩格式使用 "n" 字段，原始格式使用 "number" 字段
    let first = arr.first()?;
    if first.get("n").is_some() {
        // 压缩格式
        let compacts: Vec<CateAxisParamCompact> = serde_json::from_value(value.clone()).ok()?;
        Some(decompress_ptset(&compacts))
    } else if first.get("number").is_some() {
        // 原始格式
        serde_json::from_value(value.clone()).ok()
    } else {
        None
    }
}

/// 检测 ptset 数据是否为压缩格式
pub fn is_compressed_ptset(value: &serde_json::Value) -> bool {
    value.as_array()
        .and_then(|arr| arr.first())
        .map(|first| first.get("n").is_some())
        .unwrap_or(false)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_directions() {
        // 主轴方向
        assert!(matches!(encode_direction(Vec3::X), Vec3Id::Common(1)));
        assert!(matches!(encode_direction(Vec3::NEG_X), Vec3Id::Common(2)));
        assert!(matches!(encode_direction(Vec3::Y), Vec3Id::Common(3)));
        assert!(matches!(encode_direction(Vec3::NEG_Y), Vec3Id::Common(4)));
        assert!(matches!(encode_direction(Vec3::Z), Vec3Id::Common(5)));
        assert!(matches!(encode_direction(Vec3::NEG_Z), Vec3Id::Common(6)));
    }

    #[test]
    fn test_45_degree_directions() {
        let d45 = FRAC_1_SQRT_2;
        let dir = Vec3::new(d45, d45, 0.0);
        let id = encode_direction(dir);
        assert!(matches!(id, Vec3Id::Common(_)));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let test_dirs = [
            Vec3::X,
            Vec3::NEG_Y,
            Vec3::new(0.707, 0.707, 0.0),
            Vec3::new(0.577, 0.577, 0.577),
            Vec3::new(0.866, 0.5, 0.0), // 30°
        ];

        for dir in test_dirs {
            let normalized = dir.normalize();
            let id = encode_direction(normalized);
            let decoded = decode_direction(&id);
            let diff = (decoded - normalized).length();
            assert!(diff < 0.01, "Roundtrip failed for {:?}, diff={}", dir, diff);
        }
    }

    #[test]
    fn test_encoded_vec3_with_raw() {
        // 常见方向：不需要存储 raw
        let common = EncodedVec3::encode(Vec3::X);
        assert!(common.raw.is_none());
        assert!((common.decode() - Vec3::X).length() < 0.001);

        // 非常见方向：需要存储 raw
        let uncommon = EncodedVec3::encode(Vec3::new(0.123, 0.456, 0.789).normalize());
        assert!(uncommon.raw.is_some());
        let decoded = uncommon.decode();
        let original = Vec3::new(0.123, 0.456, 0.789).normalize();
        assert!((decoded - original).length() < 0.001);
    }

    #[test]
    fn test_zero_vector() {
        let id = encode_direction(Vec3::ZERO);
        assert!(matches!(id, Vec3Id::Zero));
        assert_eq!(decode_direction(&id), Vec3::ZERO);
    }

    #[test]
    fn test_common_direction_count() {
        let count = common_direction_count();
        println!("预定义常见方向数量: {}", count);
        assert!(count >= 26); // 至少包含 6+12+8 = 26 个基础方向
        assert!(count <= 255); // 不超过 u8 范围
    }

    #[test]
    fn test_storage_value_roundtrip() {
        let ids = [
            Vec3Id::Zero,
            Vec3Id::Common(1),
            Vec3Id::Common(100),
            Vec3Id::Hashed(12345678),
        ];

        for id in ids {
            let stored = id.to_storage_value();
            let restored = Vec3Id::from_storage_value(stored);
            assert_eq!(id, restored);
        }
    }

    #[test]
    fn test_cate_axis_param_compress() {
        // 创建测试数据
        let param = CateAxisParam {
            refno: "15196_13800".parse().unwrap_or_default(),
            number: 1,
            pt: RsVec3(Vec3::new(0.0, 0.0, 0.0)),
            dir: Some(RsVec3(Vec3::new(0.0, -1.0, 0.0))),
            dir_flag: 1.0,
            ref_dir: None,
            pbore: 50.0,
            pwidth: 0.0,
            pheight: 0.0,
            pconnect: "BOXI".to_string(),
        };

        // 压缩
        let compact = CateAxisParamCompact::from_full(&param, false);
        
        // 验证压缩格式
        assert_eq!(compact.n, 1);
        assert!(compact.d.is_some());
        assert!(compact.df.is_none()); // dir_flag=1.0 不存储
        assert!(compact.rd.is_none()); // ref_dir=None 不存储
        assert!(compact.b.is_some()); // pbore=50 存储
        assert!(compact.c.is_some()); // pconnect="BOXI" 存储
        assert!(compact.w.is_none()); // pwidth=0 不存储
        assert!(compact.h.is_none()); // pheight=0 不存储
        assert!(compact.r.is_none()); // include_refno=false

        // 解压缩
        let restored = compact.to_full();
        
        // 验证还原
        assert_eq!(restored.number, param.number);
        assert!((restored.pt.0 - param.pt.0).length() < 0.001);
        assert!((restored.dir.unwrap().0 - param.dir.unwrap().0).length() < 0.01);
        assert!((restored.pbore - param.pbore).abs() < 0.001);
        assert_eq!(restored.pconnect, param.pconnect);
    }

    #[test]
    fn test_compress_size_estimation() {
        let params = vec![
            CateAxisParam {
                refno: Default::default(),
                number: 1,
                pt: RsVec3(Vec3::ZERO),
                dir: Some(RsVec3(Vec3::Y)),
                dir_flag: 1.0,
                ref_dir: None,
                pbore: 50.0,
                pwidth: 0.0,
                pheight: 0.0,
                pconnect: "BOXI".to_string(),
            },
            CateAxisParam {
                refno: Default::default(),
                number: 2,
                pt: RsVec3(Vec3::new(0.0, 2400.0, 0.0)),
                dir: Some(RsVec3(Vec3::NEG_Y)),
                dir_flag: 1.0,
                ref_dir: None,
                pbore: 50.0,
                pwidth: 0.0,
                pheight: 0.0,
                pconnect: "BOXI".to_string(),
            },
        ];

        let original_size = estimate_original_size(&params);
        let compressed_size = estimate_compressed_size(&params);
        
        println!("原始大小: {} bytes", original_size);
        println!("压缩大小: {} bytes", compressed_size);
        println!("压缩率: {:.1}%", (1.0 - compressed_size as f64 / original_size as f64) * 100.0);
        
        assert!(compressed_size < original_size);
    }

    #[test]
    fn test_batch_compress_decompress() {
        let params = vec![
            CateAxisParam {
                refno: Default::default(),
                number: 1,
                pt: RsVec3(Vec3::ZERO),
                dir: Some(RsVec3(Vec3::X)),
                dir_flag: 1.0,
                ref_dir: None,
                pbore: 100.0,
                pwidth: 0.0,
                pheight: 0.0,
                pconnect: "".to_string(),
            },
            CateAxisParam {
                refno: Default::default(),
                number: 2,
                pt: RsVec3(Vec3::new(100.0, 200.0, 300.0)),
                dir: Some(RsVec3(Vec3::new(0.707, 0.707, 0.0))),
                dir_flag: -1.0,
                ref_dir: Some(RsVec3(Vec3::Z)),
                pbore: 0.0,
                pwidth: 50.0,
                pheight: 30.0,
                pconnect: "WELD".to_string(),
            },
        ];

        let compacts = compress_ptset(&params, true);
        let restored = decompress_ptset(&compacts);

        assert_eq!(params.len(), restored.len());
        for (orig, rest) in params.iter().zip(restored.iter()) {
            assert_eq!(orig.number, rest.number);
            assert!((orig.pt.0 - rest.pt.0).length() < 0.001);
        }
    }

    #[test]
    fn test_parse_ptset_auto_compressed() {
        // 压缩格式
        let compressed_json = serde_json::json!([
            {"n": 1, "p": [0.0, 0.0, 0.0], "d": {"id": {"Common": 3}}, "b": 50.0, "c": "BOXI"},
            {"n": 2, "p": [0.0, 2400.0, 0.0], "d": {"id": {"Common": 4}}, "b": 50.0, "c": "BOXI"}
        ]);
        
        assert!(is_compressed_ptset(&compressed_json));
        let result = parse_ptset_auto(&compressed_json);
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].number, 1);
        assert_eq!(params[1].number, 2);
    }

    #[test]
    fn test_parse_ptset_auto_original() {
        // 原始格式
        let original_json = serde_json::json!([
            {
                "refno": "15196_13800",
                "number": 1,
                "pt": [0.0, 0.0, 0.0],
                "dir": [0.0, -1.0, 0.0],
                "dir_flag": 1.0,
                "pbore": 50.0,
                "pconnect": "BOXI",
                "pwidth": 0.0,
                "pheight": 0.0
            }
        ]);
        
        assert!(!is_compressed_ptset(&original_json));
        let result = parse_ptset_auto(&original_json);
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].number, 1);
    }

    #[test]
    fn test_zero_position_omitted() {
        // 测试 ZERO 位置被省略
        let param = CateAxisParam {
            refno: Default::default(),
            number: 1,
            pt: RsVec3(Vec3::ZERO),
            dir: Some(RsVec3(Vec3::X)),
            dir_flag: 1.0,
            ref_dir: None,
            pbore: 50.0,
            pwidth: 0.0,
            pheight: 0.0,
            pconnect: "BOXI".to_string(),
        };

        let compact = CateAxisParamCompact::from_full(&param, false);
        
        // ZERO 位置应该被设为 None
        assert!(compact.p.is_none());
        
        // 序列化后不应包含 p 字段
        let json = serde_json::to_string(&compact).unwrap();
        println!("ZERO 位置序列化结果: {}", json);
        assert!(!json.contains("\"p\":"));
        
        // 还原后位置应该是 ZERO
        let restored = compact.to_full();
        assert!(restored.pt.0.length() < 0.001);
    }

    #[test]
    fn test_nonzero_position_kept() {
        // 测试非 ZERO 位置被保留
        let param = CateAxisParam {
            refno: Default::default(),
            number: 1,
            pt: RsVec3(Vec3::new(100.0, 200.0, 0.0)),
            dir: Some(RsVec3(Vec3::X)),
            dir_flag: 1.0,
            ref_dir: None,
            pbore: 50.0,
            pwidth: 0.0,
            pheight: 0.0,
            pconnect: "BOXI".to_string(),
        };

        let compact = CateAxisParamCompact::from_full(&param, false);
        
        // 非 ZERO 位置应该被保留
        assert!(compact.p.is_some());
        
        // 序列化后应包含 p 字段
        let json = serde_json::to_string(&compact).unwrap();
        println!("非 ZERO 位置序列化结果: {}", json);
        assert!(json.contains("\"p\":"));
        
        // 还原后位置应该正确
        let restored = compact.to_full();
        assert!((restored.pt.0.x - 100.0).abs() < 0.001);
        assert!((restored.pt.0.y - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_ptset_without_position() {
        // 测试解析没有 p 字段的压缩数据（位置为 ZERO）
        let compressed_json = serde_json::json!([
            {"n": 1, "d": {"id": {"Common": 3}}, "b": 50.0, "c": "BOXI"},
            {"n": 2, "p": [0.0, 2400.0, 0.0], "d": {"id": {"Common": 4}}, "b": 50.0, "c": "BOXI"}
        ]);
        
        let result = parse_ptset_auto(&compressed_json);
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.len(), 2);
        
        // 第一个点位置为 ZERO
        assert!(params[0].pt.0.length() < 0.001);
        // 第二个点位置为 [0, 2400, 0]
        assert!((params[1].pt.0.y - 2400.0).abs() < 0.001);
    }
}
