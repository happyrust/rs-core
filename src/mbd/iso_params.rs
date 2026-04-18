//! MBD iso* 系列 solver 的输入参数结构。
//!
//! 与 PML `isobran` 构造签名对齐：
//! `isobran(!name, !minslope, !maxslope, !considerprenextdir, !lookangle, 'EM4')`。
//!
//! 单位约定：所有坐标/长度均为 **毫米（mm）**，与后端 `mbd_pipe_api` 的原始坐标空间一致。
//! 方向向量统一使用右手坐标系：X=East、Y=North、Z=Up。

use glam::Vec3;

/// PML `isobran` 构造参数的 Rust 对等物。
///
/// 字段对应关系：
/// - `min_slope` / `max_slope` ↔ `!minslope` / `!maxslope`
/// - `consider_pre_next_dir` ↔ `!considerprenextdir`
/// - `look_angle` ↔ `!lookangle`
/// - `cheight` ↔ `!cheight`（字符高度，PML drawDim 的偏移基数）
/// - `em4_mode` ↔ 构造签名里的 `'EM4'` 字符串（EM4 规约下 `em4twoendtypes` 启用 two-end 处理）
#[derive(Debug, Clone)]
pub struct IsoParams {
    pub min_slope: f32,
    pub max_slope: f32,
    pub consider_pre_next_dir: bool,
    pub look_angle: f32,
    pub cheight: f32,
    pub em4_mode: bool,
}

impl Default for IsoParams {
    fn default() -> Self {
        Self {
            min_slope: 0.001,
            max_slope: 0.1,
            consider_pre_next_dir: true,
            look_angle: 60.0,
            cheight: 100.0,
            em4_mode: true,
        }
    }
}

/// 单条线性尺寸的输入语义（对应 PML 一次 `isoDim.draw` 调用所需的几何输入）。
#[derive(Debug, Clone)]
pub struct SegmentInput {
    pub id: String,
    pub kind: String,
    pub start: Vec3,
    pub end: Vec3,
    /// 管道方向（单位向量）。在 PML 里由元件 `ldir/adir` 提供；
    /// 当没有元件信息时可退到 `(end - start).normalize()`。
    pub pipe_dir: Vec3,
    /// 外径（mm）。PML isoDim 里所有 offset 都基于 OD。
    pub od: f32,
    /// 后端已经准备好的 text（保留不改，避免单位/精度 roundtrip）。
    pub text: String,
    /// 可选：该段属于哪条 isoline（lane 分配用）。
    pub isoline_index: Option<usize>,
}

/// 分支上下文，对应 PML `isobran` 对象在 `isoDim.draw` 时提供的环境。
#[derive(Debug, Clone)]
pub struct BranchContext {
    pub branch_refno: String,
    /// PML `volume of $!branname` 得到的包围盒中心点。用于 `CalculateDimChardirs`
    /// 判定"从内向外"的方向。
    pub bran_volume_center: Vec3,
    /// 当前尺寸在该分支内的"层级次数"（PML `dimtimes`，1-based）。
    /// 第一层 dimtimes=1，offset 恰好 = OD。第二层 = OD + 1.2*cheight。
    pub dim_times: u32,
}

impl BranchContext {
    /// 构造一个最小测试上下文：volume 中心位于原点，dim_times=1。
    pub fn for_test(branch_refno: impl Into<String>) -> Self {
        Self {
            branch_refno: branch_refno.into(),
            bran_volume_center: Vec3::ZERO,
            dim_times: 1,
        }
    }
}
