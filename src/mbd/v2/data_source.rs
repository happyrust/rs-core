//! MBD V2 数据源抽象层。
//!
//! 定义 V2 直算所需的数据查询接口，使 `BranchCalculatorV2` 不直接依赖
//! SurrealDB / plant-model-gen 的具体实现，便于单元测试 mock 和未来替换。
//!
//! ## 查询规范
//!
//! 实现方须遵守 AGENTS.md 中的 SurrealDB 规范：
//! - `tubi_relate` 查询用复合 ID Range
//! - 查询结果用强类型 `#[derive(SurrealValue)]`
//! - 不做全表扫描

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::primitive::Vec3V2;

/// 管段（TUBI）成员信息。
///
/// 对应 `tubi_relate` 查询结果，包含段起终点、外径、连接顺序等。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchMember {
    /// TUBI 段的 refno（使用 leave_refno）。
    pub refno: String,
    /// 所属分支 refno。
    pub owner_refno: String,
    /// 段起点（世界坐标 mm）。
    pub start: Vec3V2,
    /// 段终点（世界坐标 mm）。
    pub end: Vec3V2,
    /// 到达元件的轴线点（可选）。
    pub arrive_axis: Option<Vec3V2>,
    /// 出发元件的轴线点（可选）。
    pub leave_axis: Option<Vec3V2>,
    /// 到达元件 refno。
    pub arrive_refno: Option<String>,
    /// 连通顺序（0-based）。
    pub order: u32,
    /// 外径（mm）。
    pub outside_diameter: Option<f32>,
    /// 到达元件的 noun（类型）。
    pub arrive_noun: Option<String>,
}

impl Default for BranchMember {
    fn default() -> Self {
        Self {
            refno: String::new(),
            owner_refno: String::new(),
            start: [0.0, 0.0, 0.0],
            end: [0.0, 0.0, 0.0],
            arrive_axis: None,
            leave_axis: None,
            arrive_refno: None,
            order: 0,
            outside_diameter: None,
            arrive_noun: None,
        }
    }
}

/// 焊缝数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeldData {
    pub id: String,
    /// 焊缝位置（世界坐标 mm）。
    pub position: Vec3V2,
    /// true=车间焊（A），false=现场焊（M）。
    pub is_shop: bool,
    /// 标签文字。
    pub label: String,
    /// 左侧元件 refno。
    pub left_refno: String,
    /// 右侧元件 refno。
    pub right_refno: String,
}

/// 坡度数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlopeData {
    pub id: String,
    pub start: Vec3V2,
    pub end: Vec3V2,
    /// 有符号坡度值 `dz / horizontal`。
    pub slope: f32,
    pub text: String,
}

/// 管件标签数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagData {
    pub id: String,
    pub text: String,
    pub position: Vec3V2,
    pub noun: String,
}

/// 弯头数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BendData {
    pub id: String,
    /// 弯头顶点。
    pub vertex: Vec3V2,
    /// 弯头角度（度）。
    pub angle_deg: f32,
    /// 第一条参考方向。
    pub ray_1: Vec3V2,
    /// 第二条参考方向。
    pub ray_2: Vec3V2,
    /// 弯头处的外径（mm）。
    pub outside_diameter: Option<f32>,
}

/// 分支属性。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BranchAttrs {
    pub attrs: BTreeMap<String, String>,
    pub name: String,
}

/// V2 直算的全量管段查询结果。
///
/// 一次性拿到分支的所有标注所需数据，避免多次 DB roundtrip。
#[derive(Debug, Clone, Default)]
pub struct BranchQueryResult {
    pub members: Vec<BranchMember>,
    pub welds: Vec<WeldData>,
    pub slopes: Vec<SlopeData>,
    pub tags: Vec<TagData>,
    pub bends: Vec<BendData>,
    pub attrs: BranchAttrs,
    /// 分支包围盒中心（从 members 自动计算）。
    pub bbox_center: Option<Vec3V2>,
}

impl BranchQueryResult {
    /// 从 members 的几何数据计算包围盒中心。
    pub fn compute_bbox_center(&mut self) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        let mut has_points = false;

        for m in &self.members {
            for pt in [m.start, m.end] {
                for i in 0..3 {
                    min[i] = min[i].min(pt[i]);
                    max[i] = max[i].max(pt[i]);
                }
                has_points = true;
            }
        }

        self.bbox_center = if has_points {
            Some([
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            ])
        } else {
            None
        };
    }

    pub fn default_od(&self) -> f32 {
        self.members
            .iter()
            .filter_map(|m| m.outside_diameter)
            .next()
            .unwrap_or(229.0)
    }
}

/// 内存中的 mock 数据源，用于单元测试。
#[derive(Debug, Clone, Default)]
pub struct InMemoryDataSource {
    pub data: BTreeMap<String, BranchQueryResult>,
}

impl InMemoryDataSource {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, branch_refno: &str, result: BranchQueryResult) {
        self.data.insert(branch_refno.to_string(), result);
    }

    pub fn query(&self, branch_refno: &str) -> Option<&BranchQueryResult> {
        self.data.get(branch_refno)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_query_result_compute_bbox_center() {
        let mut result = BranchQueryResult {
            members: vec![
                BranchMember {
                    start: [0.0, 0.0, 0.0],
                    end: [1000.0, 0.0, 0.0],
                    ..Default::default()
                },
                BranchMember {
                    start: [1000.0, 0.0, 0.0],
                    end: [1000.0, 500.0, 0.0],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        result.compute_bbox_center();
        let center = result.bbox_center.unwrap();
        assert!((center[0] - 500.0).abs() < f32::EPSILON);
        assert!((center[1] - 250.0).abs() < f32::EPSILON);
        assert!((center[2] - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_members_gives_no_bbox_center() {
        let mut result = BranchQueryResult::default();
        result.compute_bbox_center();
        assert!(result.bbox_center.is_none());
    }

    #[test]
    fn default_od_uses_first_available_or_fallback() {
        let result_with_od = BranchQueryResult {
            members: vec![
                BranchMember {
                    outside_diameter: None,
                    ..Default::default()
                },
                BranchMember {
                    outside_diameter: Some(168.3),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!((result_with_od.default_od() - 168.3).abs() < f32::EPSILON);

        let result_no_od = BranchQueryResult::default();
        assert!((result_no_od.default_od() - 229.0).abs() < f32::EPSILON);
    }

    #[test]
    fn in_memory_data_source_insert_and_query() {
        let mut ds = InMemoryDataSource::new();
        let mut qr = BranchQueryResult {
            members: vec![BranchMember {
                refno: "seg-1".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [500.0, 0.0, 0.0],
                outside_diameter: Some(114.3),
                ..Default::default()
            }],
            attrs: BranchAttrs {
                name: "PIPE-001".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        qr.compute_bbox_center();
        ds.insert("=BRAN/FOO", qr);

        let loaded = ds.query("=BRAN/FOO").unwrap();
        assert_eq!(loaded.members.len(), 1);
        assert_eq!(loaded.attrs.name, "PIPE-001");
        assert!(loaded.bbox_center.is_some());

        assert!(ds.query("=BRAN/MISSING").is_none());
    }

    #[test]
    fn branch_member_serialization_roundtrip() {
        let member = BranchMember {
            refno: "24381_145712".to_string(),
            owner_refno: "=BRAN/FOO".to_string(),
            start: [100.0, 200.0, 300.0],
            end: [400.0, 500.0, 600.0],
            arrive_axis: Some([150.0, 200.0, 300.0]),
            leave_axis: None,
            arrive_refno: Some("=ELBO/BAR".to_string()),
            order: 3,
            outside_diameter: Some(168.3),
            arrive_noun: Some("ELBO".to_string()),
        };
        let json = serde_json::to_string(&member).unwrap();
        let back: BranchMember = serde_json::from_str(&json).unwrap();
        assert_eq!(back, member);
    }
}
