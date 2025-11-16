use crate::room::data_model::{RoomCode, RoomRelation, RoomRelationType, ValidationResult};
use crate::room::room_code_processor::{ProcessingResult, RoomCodeProcessor};
use crate::{RefnoEnum, SUL_DB};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

/// 数据迁移工具
///
/// 负责将现有的房间关系数据迁移到新的统一模型
pub struct MigrationTool {
    processor: RoomCodeProcessor,
    migration_stats: MigrationStats,
}

/// 迁移统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStats {
    pub total_records: usize,
    pub migrated_records: usize,
    pub failed_records: usize,
    pub skipped_records: usize,
    pub validation_errors: usize,
    pub processing_time_ms: u64,
}

/// 迁移结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub success: bool,
    pub stats: MigrationStats,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl MigrationTool {
    pub fn new() -> Self {
        Self {
            processor: RoomCodeProcessor::new(),
            migration_stats: MigrationStats {
                total_records: 0,
                migrated_records: 0,
                failed_records: 0,
                skipped_records: 0,
                validation_errors: 0,
                processing_time_ms: 0,
            },
        }
    }

    /// 迁移房间关系数据
    pub async fn migrate_room_relations(&mut self) -> anyhow::Result<MigrationResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        info!("开始迁移房间关系数据");

        // 迁移 room_relate 表
        match self.migrate_room_relate_table().await {
            Ok(stats) => {
                self.migration_stats.total_records += stats.total_records;
                self.migration_stats.migrated_records += stats.migrated_records;
                self.migration_stats.failed_records += stats.failed_records;
            }
            Err(e) => {
                errors.push(format!("迁移 room_relate 表失败: {}", e));
            }
        }

        // 迁移 room_panel_relate 表
        match self.migrate_room_panel_relate_table().await {
            Ok(stats) => {
                self.migration_stats.total_records += stats.total_records;
                self.migration_stats.migrated_records += stats.migrated_records;
                self.migration_stats.failed_records += stats.failed_records;
            }
            Err(e) => {
                errors.push(format!("迁移 room_panel_relate 表失败: {}", e));
            }
        }

        self.migration_stats.processing_time_ms = start_time.elapsed().as_millis() as u64;

        let success = errors.is_empty();
        info!(
            "迁移完成: 成功={}, 总记录={}, 迁移={}, 失败={}, 耗时={}ms",
            success,
            self.migration_stats.total_records,
            self.migration_stats.migrated_records,
            self.migration_stats.failed_records,
            self.migration_stats.processing_time_ms
        );

        Ok(MigrationResult {
            success,
            stats: self.migration_stats.clone(),
            errors,
            warnings,
        })
    }

    /// 迁移 room_relate 表
    async fn migrate_room_relate_table(&mut self) -> anyhow::Result<MigrationStats> {
        let sql = "SELECT * FROM room_relate";
        let mut response = SUL_DB.query(sql).await?;
        let records: Vec<serde_json::Value> = response.take(0)?;

        let mut stats = MigrationStats {
            total_records: records.len(),
            migrated_records: 0,
            failed_records: 0,
            skipped_records: 0,
            validation_errors: 0,
            processing_time_ms: 0,
        };

        for record in records {
            match self.migrate_single_room_relate(record).await {
                Ok(_) => stats.migrated_records += 1,
                Err(e) => {
                    stats.failed_records += 1;
                    warn!("迁移单条记录失败: {}", e);
                }
            }
        }

        Ok(stats)
    }

    /// 迁移单条 room_relate 记录
    async fn migrate_single_room_relate(
        &mut self,
        record: serde_json::Value,
    ) -> anyhow::Result<()> {
        // 解析现有记录
        let from_refno = self.extract_refno(&record, "in")?;
        let to_refno = self.extract_refno(&record, "out")?;
        let room_num = record
            .get("room_num")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        // 处理房间代码
        let processing_result = self.processor.process_room_code(room_num);
        if let Some(room_code) = processing_result.standardized_code {
            // 创建新的关系记录
            let relation = RoomRelation::new(
                RoomRelationType::RoomContains,
                from_refno,
                to_refno,
                room_code,
                0.8, // 默认置信度
            );

            // 插入新记录
            let sql = relation.to_surreal_insert();
            SUL_DB.query(&sql).await?;
        } else {
            return Err(anyhow::anyhow!("房间代码处理失败: {}", room_num));
        }

        Ok(())
    }

    /// 迁移 room_panel_relate 表
    async fn migrate_room_panel_relate_table(&mut self) -> anyhow::Result<MigrationStats> {
        let sql = "SELECT * FROM room_panel_relate";
        let mut response = SUL_DB.query(sql).await?;
        let records: Vec<serde_json::Value> = response.take(0)?;

        let mut stats = MigrationStats {
            total_records: records.len(),
            migrated_records: 0,
            failed_records: 0,
            skipped_records: 0,
            validation_errors: 0,
            processing_time_ms: 0,
        };

        for record in records {
            match self.migrate_single_room_panel_relate(record).await {
                Ok(_) => stats.migrated_records += 1,
                Err(e) => {
                    stats.failed_records += 1;
                    warn!("迁移单条面板记录失败: {}", e);
                }
            }
        }

        Ok(stats)
    }

    /// 迁移单条 room_panel_relate 记录
    async fn migrate_single_room_panel_relate(
        &mut self,
        record: serde_json::Value,
    ) -> anyhow::Result<()> {
        let from_refno = self.extract_refno(&record, "in")?;
        let to_refno = self.extract_refno(&record, "out")?;
        let room_num = record
            .get("room_num")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let processing_result = self.processor.process_room_code(room_num);
        if let Some(room_code) = processing_result.standardized_code {
            let relation = RoomRelation::new(
                RoomRelationType::RoomPanel,
                from_refno,
                to_refno,
                room_code,
                0.9, // 面板关系置信度更高
            );

            let sql = relation.to_surreal_insert();
            SUL_DB.query(&sql).await?;
        } else {
            return Err(anyhow::anyhow!("房间代码处理失败: {}", room_num));
        }

        Ok(())
    }

    /// 从记录中提取 RefnoEnum
    fn extract_refno(&self, record: &serde_json::Value, field: &str) -> anyhow::Result<RefnoEnum> {
        let refno_str = record
            .get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少字段: {}", field))?;

        // 解析 RefnoEnum
        if let Ok(refno_num) = refno_str.parse::<u64>() {
            Ok(RefnoEnum::Refno(crate::RefU64(refno_num)))
        } else {
            Err(anyhow::anyhow!("无效的 refno 格式: {}", refno_str))
        }
    }
}

/// 数据验证工具
pub struct ValidationTool;

impl ValidationTool {
    /// 验证迁移后的数据完整性
    pub async fn validate_migrated_data() -> anyhow::Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            validated_at: Utc::now(),
        };

        // 检查数据一致性
        Self::check_data_consistency(&mut result).await?;

        // 检查房间代码格式
        Self::check_room_code_formats(&mut result).await?;

        Ok(result)
    }

    /// 检查数据一致性
    async fn check_data_consistency(result: &mut ValidationResult) -> anyhow::Result<()> {
        // 检查重复关系
        let sql = r#"
            SELECT in, out, count() as cnt 
            FROM room_relate 
            GROUP BY in, out 
            HAVING cnt > 1
        "#;

        let mut response = SUL_DB.query(sql).await?;
        let duplicates: Vec<serde_json::Value> = response.take(0)?;

        if !duplicates.is_empty() {
            result.is_valid = false;
            result
                .errors
                .push(crate::room::data_model::ValidationError {
                    code: "DUPLICATE_RELATIONS".to_string(),
                    message: format!("发现 {} 个重复关系", duplicates.len()),
                    relation_id: None,
                    details: HashMap::new(),
                });
        }

        Ok(())
    }

    /// 检查房间代码格式
    async fn check_room_code_formats(result: &mut ValidationResult) -> anyhow::Result<()> {
        let sql = "SELECT DISTINCT room_code FROM room_relate WHERE room_code IS NOT NULL";
        let mut response = SUL_DB.query(sql).await?;
        let room_codes: Vec<String> = response.take(0)?;

        let mut processor = RoomCodeProcessor::new();
        let mut invalid_count = 0;

        for code in room_codes {
            let processing_result = processor.process_room_code(&code);
            if !matches!(
                processing_result.status,
                crate::room::room_code_processor::ProcessingStatus::Success
            ) {
                invalid_count += 1;
            }
        }

        if invalid_count > 0 {
            result
                .warnings
                .push(crate::room::data_model::ValidationWarning {
                    code: "INVALID_ROOM_CODES".to_string(),
                    message: format!("发现 {} 个无效房间代码", invalid_count),
                    relation_id: None,
                    suggestion: Some("运行房间代码标准化工具".to_string()),
                });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_tool_creation() {
        let tool = MigrationTool::new();
        assert_eq!(tool.migration_stats.total_records, 0);
    }
}
