//! MBD V2 图元类型定义（Phase 1）。
//!
//! 设计原则：
//! - 所有坐标都是**世界坐标、单位 mm**，与 V1 [`super::super::LayoutVec3`] 约定一致。
//! - 所有 primitive 是**已排版完成的最终输出**；前端不能再二次计算方向/偏移。
//! - 类型集合是**闭集**；新增 primitive 需要 RFC + 前后端同步升级。
//! - 顶层 [`MbdPrimitive`] 用 `#[serde(tag = "kind")]` 实现内部标签，与
//!   TypeScript 侧 `type MbdPrimitive = LinearDimPrimitive | ...` 的判别联合
//!   直接互通。
//!
//! 与 PDMS 的对应关系：
//! | 本模块 | PDMS |
//! | --- | --- |
//! | `LinearDimPrimitive` | `lindim`（`object/mbd/lindim.pmlobj`） |
//! | `AngleDimPrimitive`  | （PDMS 里靠 `lindim` 组合，这里独立一型） |
//! | `LabelPrimitive`     | `mlabel`（`object/mbd/mlabel.pmlobj`） |
//! | `LeaderLinePrimitive`| `mlabel.addleadline` 内部产生的 `type: "line"` |
//! | `AidLinePrimitive`   | `mbdaidline`（`object/mbd/mbdaidline.pmlobj`） |
//! | `AidArcPrimitive`    | `aidarc`（`object/mbd/aidarc.pmlobj`） |
//! | `AidCirclePrimitive` | `aidcircle`（`object/mbd/aidcircle.pmlobj`） |
//! | `AidPointPrimitive`  | `aidpoint`（`object/mbd/aidpoint.pmlobj`） |
//! | `AidTextPrimitive`   | `aidtext`（`object/mbd/aidtext.pmlobj`） |
//! | `WeldMarkPrimitive`  | 管道焊（`markpipe/object/isobran.pmlobj::getDimJson`） |
//! | `SlopeMarkPrimitive` | `slope` 相关（`object/mbd/lindim.pmlobj::sepSmallDim` 兼容坡度） |

use serde::{Deserialize, Serialize};

/// 世界坐标向量（mm）。与 V1 `LayoutVec3` 同构，单独别名是为了让 V2 类型
/// 独立演化，不受 V1 的兼容约束牵连。
pub type Vec3V2 = [f32; 3];

// ─────────────────────────────────────────────────────────────────────────
// 顶层响应结构
// ─────────────────────────────────────────────────────────────────────────

/// 管道 MBD V2 API 的响应封装。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MbdV2Response {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<MbdV2PipeData>,
}

/// 管道 MBD V2 数据主载荷。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MbdV2PipeData {
    /// 契约版本号。当前固定为 `"v2"`；未来 v3 会提升此值。
    pub version: String,

    /// API 请求时传入的 refno（可能是 HANG/BRAN/DB item）。
    pub input_refno: String,

    /// 实际被标注的分支 refno。
    pub branch_refno: String,

    /// 所有图元的最终列表；前端按 `kind` 分发渲染。
    pub primitives: Vec<MbdPrimitive>,

    /// 聚合元数据（仅供面板展示，不参与渲染）。
    pub meta: MbdV2Meta,

    /// 结构化问题，对标 PDMS `wronglines`。
    pub issues: Vec<MbdV2Issue>,
}

/// 管道 MBD V2 的聚合元数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MbdV2Meta {
    pub segments_count: u32,
    pub welds_count: u32,
    /// `dims_by_kind["segment"]`、`dims_by_kind["chain"]` 等。
    pub dims_by_kind: std::collections::BTreeMap<String, u32>,
    pub branch_attrs: std::collections::BTreeMap<String, String>,
    /// ISO 8601 时间戳，如 `"2026-04-21T08:12:34Z"`。
    pub generated_at: String,
}

/// 结构化问题，对标 PDMS `wronglines`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MbdV2Issue {
    pub id: String,
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_refnos: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_primitive_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Info,
    #[default]
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    #[default]
    Geometry,
    Data,
    Layout,
    Avoidance,
}

// ─────────────────────────────────────────────────────────────────────────
// Primitive 主判别联合
// ─────────────────────────────────────────────────────────────────────────

/// 已排版完成的 MBD 图元。前端按 `kind` 分发渲染。
///
/// 对应 TypeScript：
/// ```ts
/// type MbdPrimitive =
///   | LinearDimPrimitive
///   | AngleDimPrimitive
///   | LabelPrimitive
///   | LeaderLinePrimitive
///   | AidLinePrimitive
///   | AidArcPrimitive
///   | AidCirclePrimitive
///   | AidPointPrimitive
///   | AidTextPrimitive
///   | WeldMarkPrimitive
///   | SlopeMarkPrimitive;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MbdPrimitive {
    LinearDim(LinearDimPrimitive),
    AngleDim(AngleDimPrimitive),
    Label(LabelPrimitive),
    LeaderLine(LeaderLinePrimitive),
    AidLine(AidLinePrimitive),
    AidArc(AidArcPrimitive),
    AidCircle(AidCirclePrimitive),
    AidPoint(AidPointPrimitive),
    AidText(AidTextPrimitive),
    WeldMark(WeldMarkPrimitive),
    SlopeMark(SlopeMarkPrimitive),
}

impl MbdPrimitive {
    /// 统一拿到 [`CommonFields`] 里的 `id`，方便日志与交叉引用。
    pub fn id(&self) -> &str {
        match self {
            Self::LinearDim(p) => &p.common.id,
            Self::AngleDim(p) => &p.common.id,
            Self::Label(p) => &p.common.id,
            Self::LeaderLine(p) => &p.common.id,
            Self::AidLine(p) => &p.common.id,
            Self::AidArc(p) => &p.common.id,
            Self::AidCircle(p) => &p.common.id,
            Self::AidPoint(p) => &p.common.id,
            Self::AidText(p) => &p.common.id,
            Self::WeldMark(p) => &p.common.id,
            Self::SlopeMark(p) => &p.common.id,
        }
    }

    /// 是否可见。被 suppress 的 primitive `visible = false`，但仍可能留在列表里
    /// 当占位或供前端 debug 面板显示。
    pub fn visible(&self) -> bool {
        match self {
            Self::LinearDim(p) => p.common.visible,
            Self::AngleDim(p) => p.common.visible,
            Self::Label(p) => p.common.visible,
            Self::LeaderLine(p) => p.common.visible,
            Self::AidLine(p) => p.common.visible,
            Self::AidArc(p) => p.common.visible,
            Self::AidCircle(p) => p.common.visible,
            Self::AidPoint(p) => p.common.visible,
            Self::AidText(p) => p.common.visible,
            Self::WeldMark(p) => p.common.visible,
            Self::SlopeMark(p) => p.common.visible,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// 各 primitive 公共字段
// ─────────────────────────────────────────────────────────────────────────

/// 所有 primitive 都携带的公共字段。在 JSON 中通过 `#[serde(flatten)]`
/// 展平到 primitive 同层。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommonFields {
    /// 稳定唯一 ID。用于跨接口追溯、declutter、前端 debug 高亮。
    pub id: String,

    /// PDMS `nodeNames`：绑定到的模型节点名数组，供前后端按节点筛选。
    #[serde(default)]
    pub node_names: Vec<String>,

    /// 业务语义，如 `"长度"` / `"坡度"` / `"焊"`，对应 PDMS `function`。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub function: Option<String>,

    /// 产生这个 primitive 的上游对象 refno。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_refno: Option<String>,

    /// 是否可见。
    pub visible: bool,

    /// 如果 `visible = false`，此处给出抑制原因供调试。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub suppressed_reason: Option<String>,
}

impl Default for CommonFields {
    fn default() -> Self {
        Self {
            id: String::new(),
            node_names: Vec::new(),
            function: None,
            source_refno: None,
            visible: true,
            suppressed_reason: None,
        }
    }
}

/// 文字块；出现在 dim / slope / label 等处。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TextBlock {
    /// 文字基线起点（世界坐标）。
    pub anchor: Vec3V2,
    /// 文字内容，已完成格式化与单位装饰。
    pub content: String,
    /// 字高（mm），对应 PDMS `cheight`。
    pub height_mm: f32,
    /// 文字阅读方向（右向量）。
    pub orientation: Vec3V2,
    /// 文字上方向。
    pub up: Vec3V2,
}

// ─────────────────────────────────────────────────────────────────────────
// Linear dim
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinearDimSubKind {
    #[default]
    Segment,
    Chain,
    Overall,
    Port,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LinearDimArrow {
    pub position: Vec3V2,
    pub direction: Vec3V2,
}

/// 线性尺寸（对应 PDMS `lindim`）。
///
/// 几何已完全解算：`extension_1/2`、`dim_line`、`arrows`、`text.anchor`
/// 均为最终世界坐标。`level` 记录已避让的分层序号，便于 debug / QA。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LinearDimPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub sub_kind: LinearDimSubKind,
    pub extension_1: LineSegmentEndpoints,
    pub extension_2: LineSegmentEndpoints,
    pub dim_line: LineSegmentEndpoints,
    pub arrows: [LinearDimArrow; 2],
    pub text: TextBlock,
    /// 分层序号；0 为基础层，>0 表示已被 `SmallDimSolver` 错层。
    pub level: u16,
}

/// 一根直线的两个端点。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LineSegmentEndpoints {
    pub start: Vec3V2,
    pub end: Vec3V2,
}

// ─────────────────────────────────────────────────────────────────────────
// Angle dim
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AngleDimArrow {
    pub position: Vec3V2,
    /// 弧顶切线方向（单位向量），用于前端绘制弧端箭头。
    pub tangent: Vec3V2,
}

/// 角度尺寸（如弯头、管件夹角）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AngleDimPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub vertex: Vec3V2,
    /// 第一条参考射线的单位方向。
    pub ray_1: Vec3V2,
    /// 第二条参考射线的单位方向。
    pub ray_2: Vec3V2,
    pub arc: ArcGeometry,
    pub arrows: [AngleDimArrow; 2],
    pub text: TextBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ArcGeometry {
    pub center: Vec3V2,
    pub radius_mm: f32,
    pub start_angle_rad: f32,
    pub sweep_rad: f32,
    /// 弧所在平面的法线（单位向量）。
    pub normal: Vec3V2,
}

// ─────────────────────────────────────────────────────────────────────────
// Label / LeaderLine
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LabelBoxShape {
    #[default]
    None,
    Rect,
    Circle,
}

/// 带锚点的标签文字（对应 PDMS `mlabel`）。
/// 引线不在此 primitive 里，单独作为 [`LeaderLinePrimitive`] 出现。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LabelPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub anchor: Vec3V2,
    pub text_anchor: Vec3V2,
    pub content: String,
    pub height_mm: f32,
    pub orientation: Vec3V2,
    pub up: Vec3V2,
    pub box_shape: LabelBoxShape,
    /// 方框内边距（mm），仅 `box_shape != None` 时有意义。
    pub box_padding_mm: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LeaderArrowAt {
    #[default]
    None,
    Start,
    End,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LeaderLinePrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    /// 折线点，至少 2 个。
    pub points: Vec<Vec3V2>,
    pub arrow_at: LeaderArrowAt,
}

// ─────────────────────────────────────────────────────────────────────────
// Aid 辅助图元
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AidLineStyle {
    #[default]
    Solid,
    Dashed,
    DashDot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AidLinePrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    /// 折线点，至少 2 个。
    pub points: Vec<Vec3V2>,
    pub style: AidLineStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AidArcPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub center: Vec3V2,
    pub radius_mm: f32,
    pub start_angle_rad: f32,
    pub sweep_rad: f32,
    pub normal: Vec3V2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AidCirclePrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub center: Vec3V2,
    pub radius_mm: f32,
    pub normal: Vec3V2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AidPointPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub position: Vec3V2,
    pub diameter_mm: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AidTextPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub position: Vec3V2,
    pub content: String,
    pub height_mm: f32,
    pub orientation: Vec3V2,
    pub up: Vec3V2,
}

// ─────────────────────────────────────────────────────────────────────────
// Weld / Slope
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WeldType {
    #[default]
    Shop,
    Field,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WeldMarkPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub position: Vec3V2,
    pub cross_size_mm: f32,
    pub weld_type: WeldType,
    /// 可选：关联一条 [`LabelPrimitive`] 的 id；避免重复携带文字。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub linked_label_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SlopeMarkPrimitive {
    #[serde(flatten)]
    pub common: CommonFields,
    pub start: Vec3V2,
    pub end: Vec3V2,
    /// 有符号坡度：`dy / horizontal`。
    pub slope: f32,
    pub text: TextBlock,
}

// ─────────────────────────────────────────────────────────────────────────
// 测试
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_text_block() -> TextBlock {
        TextBlock {
            anchor: [0.0, 0.0, 0.0],
            content: "1234".to_string(),
            height_mm: 2.5,
            orientation: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
        }
    }

    fn sample_linear_dim() -> LinearDimPrimitive {
        LinearDimPrimitive {
            common: CommonFields {
                id: "ld-1".to_string(),
                node_names: vec!["BRAN/HANG/PIPE-001".to_string()],
                function: Some("长度".to_string()),
                source_refno: Some("=123/456".to_string()),
                visible: true,
                suppressed_reason: None,
            },
            sub_kind: LinearDimSubKind::Segment,
            extension_1: LineSegmentEndpoints {
                start: [0.0, 0.0, 0.0],
                end: [0.0, 100.0, 0.0],
            },
            extension_2: LineSegmentEndpoints {
                start: [500.0, 0.0, 0.0],
                end: [500.0, 100.0, 0.0],
            },
            dim_line: LineSegmentEndpoints {
                start: [0.0, 80.0, 0.0],
                end: [500.0, 80.0, 0.0],
            },
            arrows: [
                LinearDimArrow {
                    position: [0.0, 80.0, 0.0],
                    direction: [1.0, 0.0, 0.0],
                },
                LinearDimArrow {
                    position: [500.0, 80.0, 0.0],
                    direction: [-1.0, 0.0, 0.0],
                },
            ],
            text: TextBlock {
                anchor: [240.0, 95.0, 0.0],
                content: "500".to_string(),
                ..sample_text_block()
            },
            level: 0,
        }
    }

    #[test]
    fn primitive_kind_roundtrip_for_linear_dim() {
        let prim = MbdPrimitive::LinearDim(sample_linear_dim());
        let json = serde_json::to_value(&prim).expect("serialize");
        // 内部标签必须以 `kind: "linear_dim"` 形式出现
        assert_eq!(json["kind"], serde_json::json!("linear_dim"));
        // CommonFields 通过 #[serde(flatten)] 展平到顶层
        assert_eq!(json["id"], serde_json::json!("ld-1"));
        assert_eq!(json["visible"], serde_json::json!(true));

        let back: MbdPrimitive = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, prim);
        assert_eq!(back.id(), "ld-1");
        assert!(back.visible());
    }

    #[test]
    fn primitive_kind_roundtrip_for_weld_mark() {
        let prim = MbdPrimitive::WeldMark(WeldMarkPrimitive {
            common: CommonFields {
                id: "w-1".to_string(),
                visible: true,
                ..CommonFields::default()
            },
            position: [10.0, 20.0, 30.0],
            cross_size_mm: 50.0,
            weld_type: WeldType::Shop,
            linked_label_id: Some("lbl-1".to_string()),
        });

        let json = serde_json::to_string(&prim).expect("serialize");
        assert!(json.contains(r#""kind":"weld_mark""#));
        assert!(json.contains(r#""weld_type":"shop""#));
        assert!(json.contains(r#""linked_label_id":"lbl-1""#));

        let back: MbdPrimitive = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, prim);
    }

    #[test]
    fn primitive_kind_roundtrip_for_all_variants() {
        // 每种 primitive 构造一个最小实例，确保序列化/反序列化稳定。
        let primitives = vec![
            MbdPrimitive::LinearDim(sample_linear_dim()),
            MbdPrimitive::AngleDim(AngleDimPrimitive::default()),
            MbdPrimitive::Label(LabelPrimitive::default()),
            MbdPrimitive::LeaderLine(LeaderLinePrimitive {
                common: CommonFields::default(),
                points: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
                arrow_at: LeaderArrowAt::End,
            }),
            MbdPrimitive::AidLine(AidLinePrimitive {
                common: CommonFields::default(),
                points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                style: AidLineStyle::Dashed,
            }),
            MbdPrimitive::AidArc(AidArcPrimitive::default()),
            MbdPrimitive::AidCircle(AidCirclePrimitive::default()),
            MbdPrimitive::AidPoint(AidPointPrimitive::default()),
            MbdPrimitive::AidText(AidTextPrimitive::default()),
            MbdPrimitive::WeldMark(WeldMarkPrimitive::default()),
            MbdPrimitive::SlopeMark(SlopeMarkPrimitive::default()),
        ];

        for prim in primitives {
            let json = serde_json::to_value(&prim).expect("serialize");
            // 每个都必须带 kind 字段
            assert!(
                json.get("kind").is_some(),
                "primitive 缺少 kind 字段: {:?}",
                prim
            );
            let back: MbdPrimitive = serde_json::from_value(json).expect("deserialize");
            assert_eq!(back, prim);
        }
    }

    #[test]
    fn pipe_data_roundtrip() {
        let data = MbdV2PipeData {
            version: "v2".to_string(),
            input_refno: "=HANG/FOO".to_string(),
            branch_refno: "=BRAN/HANG/FOO".to_string(),
            primitives: vec![MbdPrimitive::LinearDim(sample_linear_dim())],
            meta: MbdV2Meta {
                segments_count: 3,
                welds_count: 2,
                dims_by_kind: {
                    let mut m = std::collections::BTreeMap::new();
                    m.insert("segment".to_string(), 3);
                    m.insert("chain".to_string(), 1);
                    m
                },
                branch_attrs: std::collections::BTreeMap::new(),
                generated_at: "2026-04-21T00:00:00Z".to_string(),
            },
            issues: vec![MbdV2Issue {
                id: "iss-1".to_string(),
                severity: IssueSeverity::Warning,
                category: IssueCategory::Geometry,
                message: "管段 OD 缺失".to_string(),
                related_refnos: vec!["=123/456".to_string()],
                related_primitive_ids: vec![],
            }],
        };

        let json = serde_json::to_string(&data).expect("serialize");
        let back: MbdV2PipeData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, data);
    }

    #[test]
    fn suppressed_reason_is_optional_and_skipped_when_none() {
        let common = CommonFields {
            id: "x".to_string(),
            visible: false,
            suppressed_reason: None,
            ..CommonFields::default()
        };
        let aid = AidPointPrimitive {
            common,
            position: [0.0, 0.0, 0.0],
            diameter_mm: 100.0,
        };
        let prim = MbdPrimitive::AidPoint(aid);
        let json = serde_json::to_string(&prim).expect("serialize");
        // `suppressed_reason` 为 None 时不应出现在 JSON 里，减小 payload
        assert!(!json.contains("suppressed_reason"));
        assert!(!json.contains("function\":null"));
    }

    #[test]
    fn enum_uses_snake_case_in_json() {
        // IssueSeverity / IssueCategory / WeldType / LabelBoxShape / LinearDimSubKind
        // 都应以 snake_case 字符串序列化，保持与 TS 侧 literal union 一致。
        assert_eq!(
            serde_json::to_value(IssueSeverity::Warning).unwrap(),
            serde_json::json!("warning")
        );
        assert_eq!(
            serde_json::to_value(IssueCategory::Avoidance).unwrap(),
            serde_json::json!("avoidance")
        );
        assert_eq!(
            serde_json::to_value(WeldType::Field).unwrap(),
            serde_json::json!("field")
        );
        assert_eq!(
            serde_json::to_value(LabelBoxShape::Rect).unwrap(),
            serde_json::json!("rect")
        );
        assert_eq!(
            serde_json::to_value(LinearDimSubKind::Overall).unwrap(),
            serde_json::json!("overall")
        );
        assert_eq!(
            serde_json::to_value(AidLineStyle::DashDot).unwrap(),
            serde_json::json!("dash_dot")
        );
    }
}
