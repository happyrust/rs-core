use crate::RefnoEnum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 统一的房间关系数据模型
///
/// 这个模块定义了房间计算系统的标准化数据模型，包括：
/// 1. 房间关系表的统一结构
/// 2. 房间代码的标准化格式
/// 3. 数据一致性验证规则
/// 4. 版本控制和变更追踪

/// 房间关系类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RoomRelationType {
    /// 房间包含构件关系
    RoomContains,
    /// 房间面板关系
    RoomPanel,
    /// 房间层级关系
    RoomHierarchy,
    /// 房间邻接关系
    RoomAdjacent,
}

impl RoomRelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoomRelationType::RoomContains => "room_relate",
            RoomRelationType::RoomPanel => "room_panel_relate",
            RoomRelationType::RoomHierarchy => "room_hierarchy_relate",
            RoomRelationType::RoomAdjacent => "room_adjacent_relate",
        }
    }
}

/// 房间代码标准格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomCode {
    /// 项目前缀 (如 "SSC", "HD", "HH")
    pub project_prefix: String,
    /// 区域代码 (如 "A", "B", "C")
    pub area_code: String,
    /// 房间号码 (如 "001", "102")
    pub room_number: String,
    /// 完整房间代码 (如 "SSC-A001", "HD-B102")
    pub full_code: String,
}

impl RoomCode {
    /// 从字符串解析房间代码
    pub fn parse(code: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = code.split('-').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("无效的房间代码格式: {}", code));
        }

        let project_prefix = parts[0].to_string();
        let room_part = parts[1];

        // 解析区域代码和房间号码
        let (area_code, room_number) = if room_part.len() == 4 {
            // 格式: A001
            (room_part[0..1].to_string(), room_part[1..].to_string())
        } else if room_part.len() == 5 {
            // 格式: A1001 -> A001
            let area = &room_part[0..1];
            let number = format!("{}{}", &room_part[1..2], &room_part[2..]);
            (area.to_string(), number)
        } else {
            return Err(anyhow::anyhow!("无效的房间号码格式: {}", room_part));
        };

        Ok(RoomCode {
            project_prefix: project_prefix.clone(),
            area_code,
            room_number,
            full_code: code.to_string(),
        })
    }

    /// 构建房间代码
    pub fn build(project_prefix: &str, area_code: &str, room_number: &str) -> Self {
        let full_code = format!("{}-{}{}", project_prefix, area_code, room_number);
        RoomCode {
            project_prefix: project_prefix.to_string(),
            area_code: area_code.to_string(),
            room_number: room_number.to_string(),
            full_code,
        }
    }

    /// 验证房间代码格式
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.project_prefix.is_empty() {
            return Err(anyhow::anyhow!("项目前缀不能为空"));
        }

        if self.area_code.len() != 1 {
            return Err(anyhow::anyhow!("区域代码必须是单个字符"));
        }

        if self.room_number.len() != 3 {
            return Err(anyhow::anyhow!("房间号码必须是3位数字"));
        }

        if !self.room_number.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!("房间号码必须是数字"));
        }

        Ok(())
    }
}

/// 统一的房间关系记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRelation {
    /// 关系唯一标识
    pub id: Uuid,
    /// 关系类型
    pub relation_type: RoomRelationType,
    /// 源节点引用号
    pub from_refno: RefnoEnum,
    /// 目标节点引用号
    pub to_refno: RefnoEnum,
    /// 房间代码
    pub room_code: RoomCode,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
    /// 空间距离 (米)
    pub spatial_distance: Option<f64>,
    /// 包含关系的几何重叠比例
    pub overlap_ratio: Option<f64>,
    /// 关系元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 创建者/系统标识
    pub created_by: String,
    /// 数据版本号
    pub version: u32,
    /// 是否有效
    pub is_active: bool,
}

impl RoomRelation {
    /// 创建新的房间关系
    pub fn new(
        relation_type: RoomRelationType,
        from_refno: RefnoEnum,
        to_refno: RefnoEnum,
        room_code: RoomCode,
        confidence: f64,
    ) -> Self {
        let now = Utc::now();
        RoomRelation {
            id: Uuid::new_v4(),
            relation_type,
            from_refno,
            to_refno,
            room_code,
            confidence,
            spatial_distance: None,
            overlap_ratio: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            created_by: "system".to_string(),
            version: 1,
            is_active: true,
        }
    }

    /// 更新关系
    pub fn update(&mut self) {
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// 设置空间信息
    pub fn with_spatial_info(mut self, distance: Option<f64>, overlap_ratio: Option<f64>) -> Self {
        self.spatial_distance = distance;
        self.overlap_ratio = overlap_ratio;
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// 验证关系数据
    pub fn validate(&self) -> anyhow::Result<()> {
        // 验证房间代码
        self.room_code.validate()?;

        // 验证置信度
        if self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(anyhow::anyhow!("置信度必须在 0.0-1.0 之间"));
        }

        // 验证空间距离
        if let Some(distance) = self.spatial_distance {
            if distance < 0.0 {
                return Err(anyhow::anyhow!("空间距离不能为负数"));
            }
        }

        // 验证重叠比例
        if let Some(ratio) = self.overlap_ratio {
            if ratio < 0.0 || ratio > 1.0 {
                return Err(anyhow::anyhow!("重叠比例必须在 0.0-1.0 之间"));
            }
        }

        Ok(())
    }

    /// 转换为 SurrealDB 插入语句
    pub fn to_surreal_insert(&self) -> String {
        let table_name = self.relation_type.as_str();
        let relation_id = format!("{}_{}", self.from_refno, self.to_refno);

        format!(
            r#"
            RELATE {}->{}:{}->{}
            SET 
                room_code = '{}',
                room_project = '{}',
                room_area = '{}',
                room_number = '{}',
                confidence = {},
                spatial_distance = {},
                overlap_ratio = {},
                metadata = {},
                created_at = '{}',
                updated_at = '{}',
                created_by = '{}',
                version = {},
                is_active = {}
            "#,
            self.from_refno.to_pe_key(),
            table_name,
            relation_id,
            self.to_refno.to_pe_key(),
            self.room_code.full_code,
            self.room_code.project_prefix,
            self.room_code.area_code,
            self.room_code.room_number,
            self.confidence,
            self.spatial_distance
                .map_or("NONE".to_string(), |d| d.to_string()),
            self.overlap_ratio
                .map_or("NONE".to_string(), |r| r.to_string()),
            serde_json::to_string(&self.metadata).unwrap_or_default(),
            self.created_at.to_rfc3339(),
            self.updated_at.to_rfc3339(),
            self.created_by,
            self.version,
            self.is_active
        )
    }
}

/// 房间关系变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRelationChange {
    /// 变更唯一标识
    pub change_id: Uuid,
    /// 关系标识
    pub relation_id: Uuid,
    /// 变更类型
    pub change_type: ChangeType,
    /// 变更前数据
    pub before_data: Option<serde_json::Value>,
    /// 变更后数据
    pub after_data: Option<serde_json::Value>,
    /// 变更原因
    pub reason: String,
    /// 变更时间
    pub changed_at: DateTime<Utc>,
    /// 变更者
    pub changed_by: String,
}

/// 变更类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
    Activate,
    Deactivate,
}

/// 房间关系统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRelationStats {
    /// 总关系数量
    pub total_relations: usize,
    /// 按类型分组的关系数量
    pub relations_by_type: HashMap<RoomRelationType, usize>,
    /// 按房间代码分组的关系数量
    pub relations_by_room: HashMap<String, usize>,
    /// 平均置信度
    pub average_confidence: f64,
    /// 低置信度关系数量 (<0.7)
    pub low_confidence_count: usize,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 数据一致性验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 规则类型
    pub rule_type: ValidationRuleType,
    /// 规则参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 是否启用
    pub enabled: bool,
}

/// 验证规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// 唯一性约束
    Uniqueness,
    /// 引用完整性
    ReferentialIntegrity,
    /// 数据格式验证
    FormatValidation,
    /// 业务逻辑验证
    BusinessLogic,
    /// 空间一致性验证
    SpatialConsistency,
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 错误信息列表
    pub errors: Vec<ValidationError>,
    /// 警告信息列表
    pub warnings: Vec<ValidationWarning>,
    /// 验证时间
    pub validated_at: DateTime<Utc>,
}

/// 验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 相关的关系ID
    pub relation_id: Option<Uuid>,
    /// 错误详情
    pub details: HashMap<String, serde_json::Value>,
}

/// 验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// 警告代码
    pub code: String,
    /// 警告消息
    pub message: String,
    /// 相关的关系ID
    pub relation_id: Option<Uuid>,
    /// 建议操作
    pub suggestion: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_code_parsing() {
        // 测试标准格式
        let code = RoomCode::parse("SSC-A001").unwrap();
        assert_eq!(code.project_prefix, "SSC");
        assert_eq!(code.area_code, "A");
        assert_eq!(code.room_number, "001");
        assert_eq!(code.full_code, "SSC-A001");

        // 测试5位格式
        let code = RoomCode::parse("HD-A1001").unwrap();
        assert_eq!(code.project_prefix, "HD");
        assert_eq!(code.area_code, "A");
        assert_eq!(code.room_number, "1001");
    }

    #[test]
    fn test_room_code_validation() {
        let valid_code = RoomCode::build("SSC", "A", "001");
        assert!(valid_code.validate().is_ok());

        let invalid_code = RoomCode::build("", "A", "001");
        assert!(invalid_code.validate().is_err());
    }

    #[test]
    fn test_room_relation_creation() {
        let room_code = RoomCode::build("SSC", "A", "001");
        let relation = RoomRelation::new(
            RoomRelationType::RoomContains,
            RefnoEnum::Refno(crate::RefU64(12345)),
            RefnoEnum::Refno(crate::RefU64(67890)),
            room_code,
            0.95,
        );

        assert_eq!(relation.confidence, 0.95);
        assert_eq!(relation.version, 1);
        assert!(relation.is_active);
        assert!(relation.validate().is_ok());
    }

    #[test]
    fn test_relation_update() {
        let room_code = RoomCode::build("SSC", "A", "001");
        let mut relation = RoomRelation::new(
            RoomRelationType::RoomContains,
            RefnoEnum::Refno(crate::RefU64(12345)),
            RefnoEnum::Refno(crate::RefU64(67890)),
            room_code,
            0.95,
        );

        let original_version = relation.version;
        relation.update();

        assert_eq!(relation.version, original_version + 1);
        assert!(relation.updated_at > relation.created_at);
    }
}
