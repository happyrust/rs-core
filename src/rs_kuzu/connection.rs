//! Kuzu 数据库连接管理
//!
//! 提供连接池、连接配置和连接生命周期管理

#[cfg(feature = "kuzu")]
use kuzu::{Connection, SystemConfig};
#[cfg(feature = "kuzu")]
use std::path::Path;

#[cfg(feature = "kuzu")]
/// Kuzu 连接配置
#[derive(Debug, Clone)]
pub struct KuzuConnectionConfig {
    /// 数据库文件路径
    pub database_path: String,
    /// 缓冲池大小（字节）
    pub buffer_pool_size: Option<u64>,
    /// 最大线程数
    pub max_num_threads: Option<u64>,
    /// 是否启用压缩
    pub enable_compression: bool,
    /// 是否只读模式
    pub read_only: bool,
}

#[cfg(feature = "kuzu")]
impl Default for KuzuConnectionConfig {
    fn default() -> Self {
        Self {
            database_path: "./data/kuzu_db".to_string(),
            buffer_pool_size: Some(4 * 1024 * 1024 * 1024), // 4GB
            max_num_threads: Some(4),                       // 默认 4 线程
            enable_compression: true,
            read_only: false,
        }
    }
}

#[cfg(feature = "kuzu")]
impl KuzuConnectionConfig {
    /// 创建新的连接配置
    pub fn new(database_path: impl Into<String>) -> Self {
        Self {
            database_path: database_path.into(),
            ..Default::default()
        }
    }

    /// 设置缓冲池大小
    pub fn with_buffer_pool_size(mut self, size: u64) -> Self {
        self.buffer_pool_size = Some(size);
        self
    }

    /// 设置最大线程数
    pub fn with_max_threads(mut self, threads: u64) -> Self {
        self.max_num_threads = Some(threads);
        self
    }

    /// 设置只读模式
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// 转换为 Kuzu SystemConfig
    pub fn to_system_config(&self) -> SystemConfig {
        let mut config = SystemConfig::default();

        if let Some(size) = self.buffer_pool_size {
            config = config.buffer_pool_size(size);
        }

        if let Some(threads) = self.max_num_threads {
            config = config.max_num_threads(threads);
        }

        config = config.enable_compression(self.enable_compression);
        config = config.read_only(self.read_only);

        config
    }

    /// 验证配置
    pub fn validate(&self) -> anyhow::Result<()> {
        // 检查路径是否有效
        let path = Path::new(&self.database_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!("数据库目录不存在: {}", parent.display()));
            }
        }

        // 验证缓冲池大小
        if let Some(size) = self.buffer_pool_size {
            if size < 1024 * 1024 {
                log::warn!("缓冲池大小过小 ({}), 建议至少 1MB", size);
            }
        }

        // 验证线程数
        if let Some(threads) = self.max_num_threads {
            if threads == 0 {
                return Err(anyhow::anyhow!("线程数不能为 0"));
            }
            if threads > 128 {
                log::warn!("线程数过大 ({}), 可能影响性能", threads);
            }
        }

        Ok(())
    }
}

#[cfg(feature = "kuzu")]
/// 连接统计信息
#[derive(Debug, Default, Clone)]
pub struct ConnectionStats {
    /// 总查询次数
    pub total_queries: u64,
    /// 失败查询次数
    pub failed_queries: u64,
    /// 平均查询时间（毫秒）
    pub avg_query_time_ms: f64,
}

#[cfg(feature = "kuzu")]
impl ConnectionStats {
    /// 记录查询
    pub fn record_query(&mut self, duration_ms: u64, success: bool) {
        self.total_queries += 1;
        if !success {
            self.failed_queries += 1;
        }

        // 计算移动平均
        let alpha = 0.1; // 平滑因子
        self.avg_query_time_ms =
            alpha * duration_ms as f64 + (1.0 - alpha) * self.avg_query_time_ms;
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_queries == 0 {
            return 1.0;
        }
        (self.total_queries - self.failed_queries) as f64 / self.total_queries as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config_default() {
        let config = KuzuConnectionConfig::default();
        assert_eq!(config.database_path, "./data/kuzu_db");
        assert!(config.buffer_pool_size.is_some());
        assert!(config.enable_compression);
    }

    #[test]
    fn test_connection_config_builder() {
        let config = KuzuConnectionConfig::new("./test_db")
            .with_buffer_pool_size(1024 * 1024 * 1024)
            .with_max_threads(4)
            .read_only(true);

        assert_eq!(config.database_path, "./test_db");
        assert_eq!(config.buffer_pool_size, Some(1024 * 1024 * 1024));
        assert_eq!(config.max_num_threads, Some(4));
        assert!(config.read_only);
    }

    #[test]
    fn test_connection_stats() {
        let mut stats = ConnectionStats::default();

        stats.record_query(100, true);
        stats.record_query(200, true);
        stats.record_query(150, false);

        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.failed_queries, 1);
        assert!(stats.success_rate() > 0.6 && stats.success_rate() < 0.7);
    }
}
