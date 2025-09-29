//! 数据库适配器配置

use serde::{Deserialize, Serialize};

/// 混合数据库模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HybridMode {
    /// SurrealDB 为主，Kuzu 为辅
    SurrealPrimary,
    /// Kuzu 为主，SurrealDB 为辅
    KuzuPrimary,
    /// 双写双读，优先 SurrealDB
    DualSurrealPreferred,
    /// 双写双读，优先 Kuzu
    DualKuzuPreferred,
    /// 写入 SurrealDB，读取 Kuzu
    WriteToSurrealReadFromKuzu,
}

impl Default for HybridMode {
    fn default() -> Self {
        Self::DualKuzuPreferred
    }
}

impl HybridMode {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "surreal_primary" => Some(Self::SurrealPrimary),
            "kuzu_primary" => Some(Self::KuzuPrimary),
            "dual_surreal_preferred" => Some(Self::DualSurrealPreferred),
            "dual_kuzu_preferred" => Some(Self::DualKuzuPreferred),
            "write_surreal_read_kuzu" | "write_to_surreal_read_from_kuzu" => {
                Some(Self::WriteToSurrealReadFromKuzu)
            }
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &str {
        match self {
            Self::SurrealPrimary => "surreal_primary",
            Self::KuzuPrimary => "kuzu_primary",
            Self::DualSurrealPreferred => "dual_surreal_preferred",
            Self::DualKuzuPreferred => "dual_kuzu_preferred",
            Self::WriteToSurrealReadFromKuzu => "write_to_surreal_read_from_kuzu",
        }
    }
}

/// 混合数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// 混合模式
    pub mode: HybridMode,
    /// 查询超时（毫秒）
    pub query_timeout_ms: u64,
    /// 是否在主数据库失败时回退到备用数据库
    pub fallback_on_error: bool,
    /// 是否启用查询缓存
    pub enable_cache: bool,
    /// 缓存过期时间（秒）
    pub cache_ttl_secs: u64,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            mode: HybridMode::default(),
            query_timeout_ms: 5000,
            fallback_on_error: true,
            enable_cache: true,
            cache_ttl_secs: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_mode_from_str() {
        assert_eq!(
            HybridMode::from_str("surreal_primary"),
            Some(HybridMode::SurrealPrimary)
        );
        assert_eq!(
            HybridMode::from_str("dual_kuzu_preferred"),
            Some(HybridMode::DualKuzuPreferred)
        );
        assert_eq!(HybridMode::from_str("invalid"), None);
    }

    #[test]
    fn test_hybrid_mode_as_str() {
        let mode = HybridMode::DualKuzuPreferred;
        assert_eq!(mode.as_str(), "dual_kuzu_preferred");
    }

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert_eq!(config.mode, HybridMode::DualKuzuPreferred);
        assert_eq!(config.query_timeout_ms, 5000);
        assert!(config.fallback_on_error);
    }
}