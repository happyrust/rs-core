use crate::room::data_model::{ChangeType, RoomRelation, RoomRelationChange};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 房间关系版本控制系统
///
/// 提供关系数据的版本管理、变更追踪和回滚功能
pub struct RoomRelationVersionControl {
    /// 变更历史记录
    change_history: Vec<RoomRelationChange>,
    /// 快照存储
    snapshots: HashMap<String, VersionSnapshot>,
    /// 当前版本号
    current_version: u64,
}

/// 版本快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSnapshot {
    /// 快照ID
    pub snapshot_id: String,
    /// 版本号
    pub version: u64,
    /// 快照时间
    pub created_at: DateTime<Utc>,
    /// 快照描述
    pub description: String,
    /// 关系数据
    pub relations: Vec<RoomRelation>,
    /// 统计信息
    pub stats: SnapshotStats,
}

/// 快照统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub total_relations: usize,
    pub relations_by_type: HashMap<String, usize>,
    pub unique_rooms: usize,
    pub data_size_bytes: usize,
}

/// 版本比较结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    /// 新增的关系
    pub added_relations: Vec<RoomRelation>,
    /// 删除的关系
    pub removed_relations: Vec<RoomRelation>,
    /// 修改的关系
    pub modified_relations: Vec<(RoomRelation, RoomRelation)>, // (old, new)
    /// 变更统计
    pub change_summary: ChangeSummary,
}

/// 变更摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSummary {
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub total_changes: usize,
}

impl RoomRelationVersionControl {
    /// 创建新的版本控制实例
    pub fn new() -> Self {
        Self {
            change_history: Vec::new(),
            snapshots: HashMap::new(),
            current_version: 0,
        }
    }

    /// 记录关系变更
    pub fn record_change(
        &mut self,
        relation_id: Uuid,
        change_type: ChangeType,
        before_data: Option<serde_json::Value>,
        after_data: Option<serde_json::Value>,
        reason: String,
        changed_by: String,
    ) {
        let change = RoomRelationChange {
            change_id: Uuid::new_v4(),
            relation_id,
            change_type,
            before_data,
            after_data,
            reason,
            changed_at: Utc::now(),
            changed_by,
        };

        self.change_history.push(change);
        debug!("记录关系变更: {:?}", relation_id);
    }

    /// 创建版本快照
    pub async fn create_snapshot(
        &mut self,
        description: String,
        relations: Vec<RoomRelation>,
    ) -> anyhow::Result<String> {
        self.current_version += 1;
        let snapshot_id = format!("snapshot_{}", self.current_version);

        // 计算统计信息
        let stats = self.calculate_snapshot_stats(&relations);

        let snapshot = VersionSnapshot {
            snapshot_id: snapshot_id.clone(),
            version: self.current_version,
            created_at: Utc::now(),
            description,
            relations,
            stats,
        };

        self.snapshots.insert(snapshot_id.clone(), snapshot);

        info!(
            "创建版本快照: {} (版本 {})",
            snapshot_id, self.current_version
        );

        Ok(snapshot_id)
    }

    /// 计算快照统计信息
    fn calculate_snapshot_stats(&self, relations: &[RoomRelation]) -> SnapshotStats {
        let mut relations_by_type = HashMap::new();
        let mut unique_rooms = std::collections::HashSet::new();

        for relation in relations {
            let type_name = format!("{:?}", relation.relation_type);
            *relations_by_type.entry(type_name).or_insert(0) += 1;
            unique_rooms.insert(relation.room_code.full_code.clone());
        }

        let data_size = serde_json::to_vec(relations).map(|v| v.len()).unwrap_or(0);

        SnapshotStats {
            total_relations: relations.len(),
            relations_by_type,
            unique_rooms: unique_rooms.len(),
            data_size_bytes: data_size,
        }
    }

    /// 获取快照
    pub fn get_snapshot(&self, snapshot_id: &str) -> Option<&VersionSnapshot> {
        self.snapshots.get(snapshot_id)
    }

    /// 列出所有快照
    pub fn list_snapshots(&self) -> Vec<&VersionSnapshot> {
        let mut snapshots: Vec<_> = self.snapshots.values().collect();
        snapshots.sort_by(|a, b| b.version.cmp(&a.version));
        snapshots
    }

    /// 比较两个版本
    pub fn compare_versions(
        &self,
        from_snapshot_id: &str,
        to_snapshot_id: &str,
    ) -> anyhow::Result<VersionDiff> {
        let from_snapshot = self
            .snapshots
            .get(from_snapshot_id)
            .ok_or_else(|| anyhow::anyhow!("快照不存在: {}", from_snapshot_id))?;

        let to_snapshot = self
            .snapshots
            .get(to_snapshot_id)
            .ok_or_else(|| anyhow::anyhow!("快照不存在: {}", to_snapshot_id))?;

        self.diff_relations(&from_snapshot.relations, &to_snapshot.relations)
    }

    /// 比较关系数据差异
    fn diff_relations(
        &self,
        old_relations: &[RoomRelation],
        new_relations: &[RoomRelation],
    ) -> anyhow::Result<VersionDiff> {
        let mut old_map: HashMap<Uuid, &RoomRelation> = HashMap::new();
        let mut new_map: HashMap<Uuid, &RoomRelation> = HashMap::new();

        for relation in old_relations {
            old_map.insert(relation.id, relation);
        }

        for relation in new_relations {
            new_map.insert(relation.id, relation);
        }

        let mut added_relations = Vec::new();
        let mut removed_relations = Vec::new();
        let mut modified_relations = Vec::new();

        // 查找新增和修改的关系
        for (id, new_relation) in &new_map {
            if let Some(old_relation) = old_map.get(id) {
                // 检查是否有修改
                if self.relations_differ(old_relation, new_relation) {
                    modified_relations.push(((*old_relation).clone(), (*new_relation).clone()));
                }
            } else {
                // 新增的关系
                added_relations.push((*new_relation).clone());
            }
        }

        // 查找删除的关系
        for (id, old_relation) in &old_map {
            if !new_map.contains_key(id) {
                removed_relations.push((*old_relation).clone());
            }
        }

        let change_summary = ChangeSummary {
            added_count: added_relations.len(),
            removed_count: removed_relations.len(),
            modified_count: modified_relations.len(),
            total_changes: added_relations.len()
                + removed_relations.len()
                + modified_relations.len(),
        };

        Ok(VersionDiff {
            added_relations,
            removed_relations,
            modified_relations,
            change_summary,
        })
    }

    /// 检查两个关系是否不同
    fn relations_differ(&self, old: &RoomRelation, new: &RoomRelation) -> bool {
        old.confidence != new.confidence
            || old.room_code != new.room_code
            || old.spatial_distance != new.spatial_distance
            || old.overlap_ratio != new.overlap_ratio
            || old.is_active != new.is_active
            || old.metadata != new.metadata
    }

    /// 回滚到指定版本
    pub async fn rollback_to_snapshot(
        &self,
        snapshot_id: &str,
    ) -> anyhow::Result<Vec<RoomRelation>> {
        let snapshot = self
            .snapshots
            .get(snapshot_id)
            .ok_or_else(|| anyhow::anyhow!("快照不存在: {}", snapshot_id))?;

        info!("回滚到快照: {} (版本 {})", snapshot_id, snapshot.version);

        Ok(snapshot.relations.clone())
    }

    /// 获取变更历史
    pub fn get_change_history(
        &self,
        relation_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Vec<&RoomRelationChange> {
        let mut changes: Vec<_> = self
            .change_history
            .iter()
            .filter(|change| relation_id.map_or(true, |id| change.relation_id == id))
            .collect();

        changes.sort_by(|a, b| b.changed_at.cmp(&a.changed_at));

        if let Some(limit) = limit {
            changes.truncate(limit);
        }

        changes
    }

    /// 清理旧的变更记录
    pub fn cleanup_old_changes(&mut self, retention_days: i64) {
        let cutoff_date = Utc::now() - chrono::Duration::days(retention_days);

        let initial_count = self.change_history.len();
        self.change_history
            .retain(|change| change.changed_at > cutoff_date);

        let removed_count = initial_count - self.change_history.len();
        if removed_count > 0 {
            info!("清理了 {} 条旧的变更记录", removed_count);
        }
    }

    /// 导出版本历史
    pub fn export_version_history(&self) -> anyhow::Result<String> {
        let export_data = VersionHistoryExport {
            snapshots: self.snapshots.values().cloned().collect(),
            change_history: self.change_history.clone(),
            current_version: self.current_version,
            exported_at: Utc::now(),
        };

        serde_json::to_string_pretty(&export_data)
            .map_err(|e| anyhow::anyhow!("导出版本历史失败: {}", e))
    }

    /// 导入版本历史
    pub fn import_version_history(&mut self, json_data: &str) -> anyhow::Result<()> {
        let import_data: VersionHistoryExport = serde_json::from_str(json_data)
            .map_err(|e| anyhow::anyhow!("解析版本历史数据失败: {}", e))?;

        // 合并快照
        for snapshot in import_data.snapshots {
            self.snapshots
                .insert(snapshot.snapshot_id.clone(), snapshot);
        }

        // 合并变更历史
        self.change_history.extend(import_data.change_history);

        // 更新版本号
        self.current_version = self.current_version.max(import_data.current_version);

        info!("成功导入版本历史数据");
        Ok(())
    }
}

/// 版本历史导出数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionHistoryExport {
    snapshots: Vec<VersionSnapshot>,
    change_history: Vec<RoomRelationChange>,
    current_version: u64,
    exported_at: DateTime<Utc>,
}

/// 全局版本控制实例
static GLOBAL_VERSION_CONTROL: tokio::sync::OnceCell<
    tokio::sync::Mutex<RoomRelationVersionControl>,
> = tokio::sync::OnceCell::const_new();

/// 获取全局版本控制实例
pub async fn get_global_version_control() -> &'static tokio::sync::Mutex<RoomRelationVersionControl>
{
    GLOBAL_VERSION_CONTROL
        .get_or_init(|| async { tokio::sync::Mutex::new(RoomRelationVersionControl::new()) })
        .await
}

/// 便捷函数：记录关系变更
pub async fn record_relation_change(
    relation_id: Uuid,
    change_type: ChangeType,
    before_data: Option<serde_json::Value>,
    after_data: Option<serde_json::Value>,
    reason: String,
    changed_by: String,
) {
    let vc = get_global_version_control().await;
    let mut vc = vc.lock().await;
    vc.record_change(
        relation_id,
        change_type,
        before_data,
        after_data,
        reason,
        changed_by,
    );
}

/// 便捷函数：创建快照
pub async fn create_relation_snapshot(
    description: String,
    relations: Vec<RoomRelation>,
) -> anyhow::Result<String> {
    let vc = get_global_version_control().await;
    let mut vc = vc.lock().await;
    vc.create_snapshot(description, relations).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RefnoEnum;
    use crate::room::data_model::{RoomCode, RoomRelationType};

    #[test]
    fn test_version_control_creation() {
        let vc = RoomRelationVersionControl::new();
        assert_eq!(vc.current_version, 0);
        assert!(vc.snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_creation() {
        let mut vc = RoomRelationVersionControl::new();

        let room_code = RoomCode::build("SSC", "A", "001");
        let relation = crate::room::data_model::RoomRelation::new(
            RoomRelationType::RoomContains,
            RefnoEnum::Refno(crate::RefU64(12345)),
            RefnoEnum::Refno(crate::RefU64(67890)),
            room_code,
            0.95,
        );

        let snapshot_id = vc
            .create_snapshot("测试快照".to_string(), vec![relation])
            .await
            .unwrap();

        assert_eq!(vc.current_version, 1);
        assert!(vc.snapshots.contains_key(&snapshot_id));
    }

    #[test]
    fn test_change_recording() {
        let mut vc = RoomRelationVersionControl::new();
        let relation_id = Uuid::new_v4();

        vc.record_change(
            relation_id,
            ChangeType::Create,
            None,
            Some(serde_json::json!({"test": "data"})),
            "测试创建".to_string(),
            "test_user".to_string(),
        );

        assert_eq!(vc.change_history.len(), 1);
        assert_eq!(vc.change_history[0].relation_id, relation_id);
    }
}
