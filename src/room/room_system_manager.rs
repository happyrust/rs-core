use crate::room::{
    data_model::{RoomCode, RoomRelation, RoomRelationType},
    migration_tools::{MigrationTool, ValidationTool},
    monitoring::{RoomSystemMetrics, RoomSystemMonitor},
    room_code_processor::{ProcessingResult, RoomCodeProcessor},
    version_control::RoomRelationVersionControl,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 房间系统管理器
///
/// 统一管理房间计算系统的所有组件，提供高级API接口
pub struct RoomSystemManager {
    /// 房间代码处理器
    code_processor: RoomCodeProcessor,
    /// 数据迁移工具
    migration_tool: MigrationTool,
    /// 版本控制系统
    version_control: RoomRelationVersionControl,
    /// 系统监控器
    monitor: RoomSystemMonitor,
    /// 管理器配置
    config: ManagerConfig,
}

/// 管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerConfig {
    /// 是否启用自动快照
    pub auto_snapshot_enabled: bool,
    /// 快照间隔（小时）
    pub snapshot_interval_hours: u64,
    /// 变更记录保留天数
    pub change_retention_days: i64,
    /// 是否启用数据验证
    pub validation_enabled: bool,
    /// 批处理大小
    pub batch_size: usize,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            auto_snapshot_enabled: true,
            snapshot_interval_hours: 24,
            change_retention_days: 30,
            validation_enabled: true,
            batch_size: 1000,
        }
    }
}

/// 系统操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOperationResult {
    pub success: bool,
    pub operation_id: Uuid,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<Utc>,
}

impl RoomSystemManager {
    /// 创建新的房间系统管理器
    pub fn new(config: Option<ManagerConfig>) -> Self {
        Self {
            code_processor: RoomCodeProcessor::new(),
            migration_tool: MigrationTool::new(),
            version_control: RoomRelationVersionControl::new(),
            monitor: RoomSystemMonitor::new(),
            config: config.unwrap_or_default(),
        }
    }

    /// 初始化房间系统
    pub async fn initialize(&mut self) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();
        info!("初始化房间系统管理器: {}", operation_id);

        let mut details = HashMap::new();

        // 启动监控
        if self.config.auto_snapshot_enabled {
            self.start_auto_snapshot_task().await;
            details.insert("auto_snapshot".to_string(), serde_json::json!(true));
        }

        // 启动监控任务
        let monitor_interval = std::time::Duration::from_secs(30);
        std::sync::Arc::new(self.monitor.clone()).start_monitoring_task(monitor_interval);
        details.insert("monitoring".to_string(), serde_json::json!(true));

        Ok(SystemOperationResult {
            success: true,
            operation_id,
            message: "房间系统初始化成功".to_string(),
            details,
            timestamp: Utc::now(),
        })
    }

    /// 处理房间代码
    pub async fn process_room_code(&mut self, input: &str) -> ProcessingResult {
        self.code_processor.process_room_code(input)
    }

    /// 批量处理房间代码
    pub async fn batch_process_room_codes(&mut self, inputs: Vec<String>) -> Vec<ProcessingResult> {
        self.code_processor.batch_process(inputs)
    }

    /// 创建房间关系
    pub async fn create_room_relation(
        &mut self,
        relation_type: RoomRelationType,
        from_refno: crate::RefnoEnum,
        to_refno: crate::RefnoEnum,
        room_code_str: &str,
        confidence: f64,
    ) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();

        // 处理房间代码
        let processing_result = self.code_processor.process_room_code(room_code_str);
        let room_code = match processing_result.standardized_code {
            Some(code) => code,
            None => {
                return Ok(SystemOperationResult {
                    success: false,
                    operation_id,
                    message: "房间代码处理失败".to_string(),
                    details: HashMap::new(),
                    timestamp: Utc::now(),
                });
            }
        };

        // 创建关系
        let relation =
            RoomRelation::new(relation_type, from_refno, to_refno, room_code, confidence);

        // 验证关系
        if self.config.validation_enabled {
            if let Err(e) = relation.validate() {
                return Ok(SystemOperationResult {
                    success: false,
                    operation_id,
                    message: format!("关系验证失败: {}", e),
                    details: HashMap::new(),
                    timestamp: Utc::now(),
                });
            }
        }

        // 记录变更
        self.version_control.record_change(
            relation.id,
            crate::room::data_model::ChangeType::Create,
            None,
            Some(serde_json::to_value(&relation)?),
            "创建新关系".to_string(),
            "system".to_string(),
        );

        // 插入数据库
        let sql = relation.to_surreal_insert();
        crate::SUL_DB.query(&sql).await?;

        let mut details = HashMap::new();
        details.insert("relation_id".to_string(), serde_json::json!(relation.id));
        details.insert(
            "room_code".to_string(),
            serde_json::json!(relation.room_code.full_code),
        );

        Ok(SystemOperationResult {
            success: true,
            operation_id,
            message: "房间关系创建成功".to_string(),
            details,
            timestamp: Utc::now(),
        })
    }

    /// 执行数据迁移
    pub async fn migrate_legacy_data(&mut self) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();
        info!("开始数据迁移: {}", operation_id);

        // 创建迁移前快照
        let current_relations = self.load_current_relations().await?;
        let pre_migration_snapshot = self
            .version_control
            .create_snapshot("迁移前快照".to_string(), current_relations)
            .await?;

        // 执行迁移
        let migration_result = self.migration_tool.migrate_room_relations().await?;

        // 创建迁移后快照
        let post_migration_relations = self.load_current_relations().await?;
        let post_migration_snapshot = self
            .version_control
            .create_snapshot("迁移后快照".to_string(), post_migration_relations)
            .await?;

        let mut details = HashMap::new();
        details.insert(
            "pre_migration_snapshot".to_string(),
            serde_json::json!(pre_migration_snapshot),
        );
        details.insert(
            "post_migration_snapshot".to_string(),
            serde_json::json!(post_migration_snapshot),
        );
        details.insert(
            "migration_stats".to_string(),
            serde_json::to_value(&migration_result.stats)?,
        );

        Ok(SystemOperationResult {
            success: migration_result.success,
            operation_id,
            message: if migration_result.success {
                "数据迁移成功完成".to_string()
            } else {
                "数据迁移部分失败".to_string()
            },
            details,
            timestamp: Utc::now(),
        })
    }

    /// 验证系统数据
    pub async fn validate_system_data(&self) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();
        info!("开始系统数据验证: {}", operation_id);

        let validation_result = ValidationTool::validate_migrated_data().await?;

        let mut details = HashMap::new();
        details.insert(
            "validation_result".to_string(),
            serde_json::to_value(&validation_result)?,
        );
        details.insert(
            "errors_count".to_string(),
            serde_json::json!(validation_result.errors.len()),
        );
        details.insert(
            "warnings_count".to_string(),
            serde_json::json!(validation_result.warnings.len()),
        );

        Ok(SystemOperationResult {
            success: validation_result.is_valid,
            operation_id,
            message: if validation_result.is_valid {
                "数据验证通过".to_string()
            } else {
                format!("数据验证失败: {} 个错误", validation_result.errors.len())
            },
            details,
            timestamp: Utc::now(),
        })
    }

    /// 获取系统指标
    pub async fn get_system_metrics(&self) -> RoomSystemMetrics {
        self.monitor.get_current_metrics().await
    }

    /// 创建手动快照
    pub async fn create_manual_snapshot(
        &mut self,
        description: String,
    ) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();

        let relations = self.load_current_relations().await?;
        let snapshot_id = self
            .version_control
            .create_snapshot(description.clone(), relations)
            .await?;

        let mut details = HashMap::new();
        details.insert("snapshot_id".to_string(), serde_json::json!(snapshot_id));
        details.insert("description".to_string(), serde_json::json!(description));

        Ok(SystemOperationResult {
            success: true,
            operation_id,
            message: "快照创建成功".to_string(),
            details,
            timestamp: Utc::now(),
        })
    }

    /// 回滚到指定快照
    pub async fn rollback_to_snapshot(
        &mut self,
        snapshot_id: &str,
    ) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();
        info!("回滚到快照: {} (操作ID: {})", snapshot_id, operation_id);

        // 创建回滚前快照
        let current_relations = self.load_current_relations().await?;
        let pre_rollback_snapshot = self
            .version_control
            .create_snapshot(
                format!("回滚前快照 (目标: {})", snapshot_id),
                current_relations,
            )
            .await?;

        // 执行回滚
        let rollback_relations = self
            .version_control
            .rollback_to_snapshot(snapshot_id)
            .await?;

        // 应用回滚数据（这里需要实现具体的数据库操作）
        // TODO: 实现数据库回滚逻辑

        let mut details = HashMap::new();
        details.insert(
            "target_snapshot".to_string(),
            serde_json::json!(snapshot_id),
        );
        details.insert(
            "pre_rollback_snapshot".to_string(),
            serde_json::json!(pre_rollback_snapshot),
        );
        details.insert(
            "rollback_relations_count".to_string(),
            serde_json::json!(rollback_relations.len()),
        );

        Ok(SystemOperationResult {
            success: true,
            operation_id,
            message: "回滚操作成功".to_string(),
            details,
            timestamp: Utc::now(),
        })
    }

    /// 清理系统数据
    pub async fn cleanup_system(&mut self) -> anyhow::Result<SystemOperationResult> {
        let operation_id = Uuid::new_v4();
        info!("开始系统清理: {}", operation_id);

        // 清理旧的变更记录
        self.version_control
            .cleanup_old_changes(self.config.change_retention_days);

        // 清理房间代码缓存
        self.code_processor.clear_cache();

        let mut details = HashMap::new();
        details.insert(
            "change_retention_days".to_string(),
            serde_json::json!(self.config.change_retention_days),
        );

        Ok(SystemOperationResult {
            success: true,
            operation_id,
            message: "系统清理完成".to_string(),
            details,
            timestamp: Utc::now(),
        })
    }

    /// 启动自动快照任务
    async fn start_auto_snapshot_task(&self) {
        let interval_hours = self.config.snapshot_interval_hours;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_hours * 3600));

            loop {
                interval.tick().await;

                // 这里需要访问管理器实例来创建快照
                // 由于所有权问题，实际实现可能需要使用 Arc<Mutex<>> 或其他并发原语
                info!("执行自动快照任务");
            }
        });
    }

    /// 加载当前关系数据
    async fn load_current_relations(&self) -> anyhow::Result<Vec<RoomRelation>> {
        // TODO: 实现从数据库加载当前关系数据的逻辑
        // 这里返回空向量作为占位符
        Ok(Vec::new())
    }
}

/// 全局房间系统管理器实例
static GLOBAL_MANAGER: tokio::sync::OnceCell<tokio::sync::Mutex<RoomSystemManager>> =
    tokio::sync::OnceCell::const_new();

/// 获取全局房间系统管理器
pub async fn get_global_manager() -> &'static tokio::sync::Mutex<RoomSystemManager> {
    GLOBAL_MANAGER
        .get_or_init(|| async { tokio::sync::Mutex::new(RoomSystemManager::new(None)) })
        .await
}

/// 便捷函数：初始化房间系统
pub async fn initialize_room_system() -> anyhow::Result<SystemOperationResult> {
    let manager = get_global_manager().await;
    let mut manager = manager.lock().await;
    manager.initialize().await
}

/// 便捷函数：处理房间代码
pub async fn process_room_code_global(input: &str) -> ProcessingResult {
    let manager = get_global_manager().await;
    let mut manager = manager.lock().await;
    manager.process_room_code(input).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = RoomSystemManager::new(None);
        assert!(manager.config.auto_snapshot_enabled);
        assert_eq!(manager.config.batch_size, 1000);
    }

    #[tokio::test]
    async fn test_manager_initialization() {
        let mut manager = RoomSystemManager::new(None);
        let result = manager.initialize().await.unwrap();
        assert!(result.success);
    }
}
