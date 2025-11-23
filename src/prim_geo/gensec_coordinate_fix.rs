//! GENSEC坐标修复模块
//! 
//! 用于修复GENSEC元素坐标系转换问题，确保POINSP等子元素的坐标计算正确

use glam::DVec3;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::OnceLock;

/// GENSEC坐标修复配置
#[derive(Debug, Clone)]
pub struct GensecCoordinateFixConfig {
    /// 是否启用GENSEC坐标修复
    pub enabled: bool,
    /// 是否应用符号反转（X和Z轴）
    pub apply_sign_flip: bool,
    /// 是否应用SPINE偏移修正
    pub apply_spine_offset: bool,
    /// 是否启用日志记录
    pub enable_logging: bool,
}

impl Default for GensecCoordinateFixConfig {
    fn default() -> Self {
        Self {
            enabled: false, // 默认禁用，采用白名单策略
            apply_sign_flip: true,
            apply_spine_offset: true,
            enable_logging: true,
        }
    }
}

/// GENSEC坐标修复管理器
pub struct GensecCoordinateFixManager {
    /// 白名单：允许应用修复的GENSEC元素
    whitelist: HashSet<String>,
    /// 全局配置
    config: GensecCoordinateFixConfig,
}

impl GensecCoordinateFixManager {
    /// 创建新的修复管理器
    pub fn new() -> Self {
        let mut whitelist = HashSet::new();
        
        // 初始化白名单，只包含已验证的GENSEC元素
        whitelist.insert("17496_266217".to_string());
        
        Self {
            whitelist,
            config: GensecCoordinateFixConfig::default(),
        }
    }
    
    /// 从环境变量加载配置
    pub fn load_from_env(mut self) -> Self {
        // 检查是否全局启用修复
        if let Ok(enabled_str) = std::env::var("GENSEC_COORD_FIX_ENABLED") {
            self.config.enabled = enabled_str.parse().unwrap_or(false);
        }
        
        // 检查是否启用日志
        if let Ok(logging_str) = std::env::var("GENSEC_COORD_FIX_LOGGING") {
            self.config.enable_logging = logging_str.parse().unwrap_or(true);
        }
        
        // 检查额外的白名单元素
        if let Ok(whitelist_str) = std::env::var("GENSEC_COORD_FIX_WHITELIST") {
            for refno in whitelist_str.split(',') {
                let refno = refno.trim().to_string();
                if !refno.is_empty() {
                    self.whitelist.insert(refno);
                }
            }
        }
        
        self
    }
    
    /// 检查指定GENSEC元素是否允许应用修复
    pub fn is_allowed(&self, gensec_refno: &str) -> bool {
        self.config.enabled && self.whitelist.contains(gensec_refno)
    }
    
    /// 修复GENSEC坐标（带安全检查和日志）
    pub fn fix_coordinates(&self, gensec_pos: DVec3, spine_offset: DVec3, gensec_refno: &str) -> Option<DVec3> {
        if !self.is_allowed(gensec_refno) {
            return None;
        }
        
        let original_pos = gensec_pos;
        let fixed_pos = self.apply_fix(gensec_pos, spine_offset);
        
        // 记录修复日志
        if self.config.enable_logging {
            self.log_fix(gensec_refno, original_pos, spine_offset, fixed_pos);
        }
        
        // 验证修复结果
        if !is_reasonable_coordinate(fixed_pos) {
            eprintln!("⚠️  GENSEC {} 修复结果不合理: ({:.3}, {:.3}, {:.3})", 
                     gensec_refno, fixed_pos.x, fixed_pos.y, fixed_pos.z);
            return None;
        }
        
        Some(fixed_pos)
    }
    
    /// 应用实际的坐标修复逻辑
    fn apply_fix(&self, gensec_pos: DVec3, _spine_offset: DVec3) -> DVec3 {
        let mut fixed_pos = gensec_pos;
        
        // 应用符号反转（X和Z轴）
        if self.config.apply_sign_flip {
            fixed_pos.x = -fixed_pos.x;
            fixed_pos.z = -fixed_pos.z;
        }
        
        // 注意：不再应用SPINE偏移修正，避免重复应用
        // SPINE偏移应该只在矩阵变换时应用一次
        
        fixed_pos
    }
    
    /// 记录修复日志
    fn log_fix(&self, gensec_refno: &str, original: DVec3, spine_offset: DVec3, fixed: DVec3) {
        println!("🔧 GENSEC坐标修复: {}", gensec_refno);
        println!("  原始位置: ({:.6}, {:.6}, {:.6})", original.x, original.y, original.z);
        println!("  SPINE偏移: ({:.6}, {:.6}, {:.6})", spine_offset.x, spine_offset.y, spine_offset.z);
        println!("  修复位置: ({:.6}, {:.6}, {:.6})", fixed.x, fixed.y, fixed.z);
        
        let diff = (fixed - original).length();
        println!("  修正幅度: {:.6} mm", diff);
    }
    
    /// 添加GENSEC元素到白名单
    pub fn add_to_whitelist(&mut self, gensec_refno: String) {
        self.whitelist.insert(gensec_refno);
    }
    
    /// 获取当前白名单
    pub fn get_whitelist(&self) -> &HashSet<String> {
        &self.whitelist
    }
}

/// 验证修复结果的合理性
pub fn is_reasonable_coordinate(fixed_pos: DVec3) -> bool {
    // 坐标应该在合理的范围内（非零且不太大）
    let threshold = 100.0; // 0.1mm阈值
    let max_threshold = 100000.0; // 100km最大值
    
    fixed_pos.x.abs() > threshold 
        && fixed_pos.y.abs() > threshold 
        && fixed_pos.z.abs() > threshold
        && fixed_pos.x.abs() < max_threshold
        && fixed_pos.y.abs() < max_threshold
        && fixed_pos.z.abs() < max_threshold
}

/// 全局修复管理器实例（线程安全的延迟初始化）
static GENSEC_FIX_MANAGER: OnceLock<GensecCoordinateFixManager> = OnceLock::new();

/// 获取全局修复管理器
pub fn get_fix_manager() -> &'static GensecCoordinateFixManager {
    GENSEC_FIX_MANAGER.get_or_init(|| GensecCoordinateFixManager::new().load_from_env())
}

/// 便捷函数：修复指定GENSEC元素的坐标
pub fn fix_gensec_coordinates_safe(
    gensec_pos: DVec3,
    spine_offset: DVec3,
    gensec_refno: &str,
) -> Option<DVec3> {
    get_fix_manager().fix_coordinates(gensec_pos, spine_offset, gensec_refno)
}

/// 修复GENSEC坐标的专用函数（保持向后兼容）
/// 
/// # 参数
/// - `gensec_pos`: GENSEC元素的原始位置
/// - `spine_offset`: SPINE偏移向量（通常是SPINE[1]坐标）
/// - `config`: 修复配置
/// 
/// # 返回值
/// 修复后的GENSEC位置坐标
#[deprecated(note = "使用 fix_gensec_coordinates_safe 替代")]
pub fn fix_gensec_coordinates(
    gensec_pos: DVec3,
    spine_offset: DVec3,
    config: &GensecCoordinateFixConfig,
) -> DVec3 {
    if !config.enabled {
        return gensec_pos;
    }
    
    let mut fixed_pos = gensec_pos;
    
    // 应用符号反转（X和Z轴）
    if config.apply_sign_flip {
        fixed_pos.x = -fixed_pos.x;
        fixed_pos.z = -fixed_pos.z;
    }
    
    // 应用SPINE偏移修正
    if config.apply_spine_offset {
        fixed_pos.x -= spine_offset.x;
        fixed_pos.y += spine_offset.y; // Y轴保持不变但加上偏移
        fixed_pos.z -= spine_offset.z;
    }
    
    fixed_pos
}

/// 获取针对特定GENSEC元素的修复配置
/// 
/// # 参数
/// - `gensec_refno`: GENSEC元素的引用号
/// 
/// # 返回值
/// 该GENSEC元素的修复配置
#[deprecated(note = "使用白名单机制替代")]
pub fn get_gensec_fix_config(gensec_refno: &str) -> GensecCoordinateFixConfig {
    // 目前对所有GENSEC元素使用相同配置
    // 未来可以根据特定元素调整配置
    match gensec_refno {
        "17496_266217" => {
            // 目标GENSEC元素的特殊配置
            GensecCoordinateFixConfig {
                enabled: true,
                apply_sign_flip: true,
                apply_spine_offset: true,
                enable_logging: true,
            }
        }
        _ => {
            // 默认配置
            GensecCoordinateFixConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_gensec_coordinates() {
        let gensec_pos = DVec3::new(-5375.0, 1148.699951, -2595.689941);
        let spine_offset = DVec3::new(-0.490000, 622.590027, -11.320000);
        let config = GensecCoordinateFixConfig::default();
        
        let fixed = fix_gensec_coordinates(gensec_pos, spine_offset, &config);
        
        // 验证修复结果
        assert!(is_reasonable_coordinate(fixed));
        
        // 验证接近期望值
        let expected = DVec3::new(5375.49, 1771.29, 2607.01);
        let diff = (fixed - expected).length();
        assert!(diff < 1.0, "差异过大: {:.6}mm", diff);
    }

    #[test]
    fn test_disabled_fix() {
        let gensec_pos = DVec3::new(-5375.0, 1148.699951, -2595.689941);
        let spine_offset = DVec3::new(-0.490000, 622.590027, -11.320000);
        let config = GensecCoordinateFixConfig { enabled: false, ..Default::default() };
        
        let fixed = fix_gensec_coordinates(gensec_pos, spine_offset, &config);
        
        // 禁用时应返回原始值
        assert_eq!(fixed, gensec_pos);
    }
}
