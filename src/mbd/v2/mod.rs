//! MBD V2 数据契约模块。
//!
//! V2 的核心设计：后端产出「已排版完成的图元列表」，前端只做 primitive
//! 到渲染实体的 1:1 映射。这与 PDMS 的 `json.getJson()` / `jsonmem.getjsontext`
//! 同构（见 `rs-core/MBD/object/mbd/json.pmlobj`）。
//!
//! 详见 `rs-core/MBD/开发文档/MBD-V2-开发计划.md`。
//!
//! 本模块在 Phase 1 只提供**类型定义**；排版算法、避让、组装 API 将在
//! Phase 2–3 陆续加入。
//!
//! 示例：
//! ```rust
//! use aios_core::mbd::v2::{MbdV2PipeData, MbdV2Meta};
//!
//! let data = MbdV2PipeData {
//!     version: "v2".to_string(),
//!     input_refno: "=BRAN/HANG/foo".to_string(),
//!     branch_refno: "=BRAN/HANG/foo".to_string(),
//!     primitives: vec![],
//!     meta: MbdV2Meta::default(),
//!     issues: vec![],
//! };
//! assert_eq!(data.version, "v2");
//! ```

pub mod assembler;
pub mod avoidance;
pub mod leader_router;
pub mod pipeline;
pub mod primitive;
pub mod small_dim;
pub mod text_measurement;

pub use assembler::{
    AssemblerContext, ChainGroup, ChainTolerance, LinearDimChain, SmallDimChainParams,
    assemble_v2_primitives, assemble_v2_primitives_with_chain_stacking,
    expand_linear_dim_chain, group_dims_into_chains,
};
pub use avoidance::{
    AvoidanceConfig, detect_leader_line_label_conflicts, reroute_leader_lines_around_labels,
    resolve_label_label_conflicts,
};
pub use pipeline::{MbdV2PipelineContext, build_mbd_v2_pipe_data};
pub use primitive::{
    AidArcPrimitive, AidCirclePrimitive, AidLinePrimitive, AidLineStyle, AidPointPrimitive,
    AidTextPrimitive, AngleDimArrow, AngleDimPrimitive, IssueCategory, IssueSeverity,
    LabelBoxShape, LabelPrimitive, LeaderArrowAt, LeaderLinePrimitive, LinearDimArrow,
    LinearDimPrimitive, LinearDimSubKind, MbdPrimitive, MbdV2Issue, MbdV2Meta, MbdV2PipeData,
    MbdV2Response, SlopeMarkPrimitive, TextBlock, Vec3V2, WeldMarkPrimitive, WeldType,
};
