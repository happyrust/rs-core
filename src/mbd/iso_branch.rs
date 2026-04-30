//! Branch 级协调：对标 PML [`isobran.pmlobj`](../../MBD/markpipe/object/isobran.pmlobj) 的
//! `split() / getisolines() / putIntoIsoLine()` 语义，同时实现
//! [`isoUsedDir.pmlobj`](../../MBD/markpipe/object/isoUsedDir.pmlobj) 的"方向已用计数"。
//!
//! 本模块做两件事：
//!
//! 1. `UsedDirRegistry`：记录每条 linear dim 的 `(direction, dis_min, dis_max)` 区间，
//!    判定同方向、同距离区间的 dim 是否冲突，若冲突则自动把后来的 dim 推到下一个 lane
//!    （对应 PML `dimtimes` 自增，offset += 1.2*cheight）。
//! 2. `solve_linear_dim_series`：批量求解一组 [`SegmentInput`] 的线性尺寸，自动调用
//!    registry 分配 `dim_times`，返回带 lane 的 `PlacedLinearDim` 列表。
//!
//! 未实现（留给后续迭代）：
//! - `isobran.split()` 的拓扑级切分（按弯头/三通/焊口分 isoline）— 当前 MVP 假设输入
//!   已经是"单一 isoline"的有序 dim 序列。
//! - `isoUsedDir` 对 lookangle / slope 的加权惩罚。

use glam::Vec3;

use crate::mbd::PlacedLinearDim;
use crate::mbd::iso_dim::{angle_deg, compute_linear_dim_layout};
use crate::mbd::iso_params::{BranchContext, IsoParams, SegmentInput};

const DIRECTION_ALIGN_COS_THRESHOLD: f32 = 0.94; // ~20° 内视为同方向（含反向）
const DISTANCE_OVERLAP_TOLERANCE_MM: f32 = 0.5;
/// 两条 dim 的中点在"dim_dir 法平面"上的距离阈值：小于该值视为空间邻近，
/// 需要 lane 阶梯；大于该值则即使方向、投影区间相似也不冲突（不同段管道）。
/// 经验值：按 1/4 个 dim 长度与一个典型 cheight 的较大者。
const SPATIAL_PROXIMITY_FACTOR: f32 = 0.25;
const SPATIAL_PROXIMITY_FLOOR_MM: f32 = 300.0;

/// 一次"方向已用"登记，对应 PML `object isoUsedDir(!mem, !dir, !mindis, !maxdis, !kind)`。
#[derive(Debug, Clone)]
pub struct UsedDirEntry {
    pub direction: Vec3,
    pub dis_min: f32,
    pub dis_max: f32,
    pub kind: String,
    pub dim_times: u32,
    /// dim 在 world 空间中的中点（用于空间邻近性检测，避免把不同段管道的同向
    /// dim 误判为冲突）。
    pub mid: Vec3,
    /// dim 长度（mm），用作邻近阈值的动态尺度。
    pub length: f32,
}

/// 方向已用登记表，供 `solve_linear_dim_series` 遍历时检测 lane 冲突。
#[derive(Debug, Default)]
pub struct UsedDirRegistry {
    entries: Vec<UsedDirEntry>,
}

impl UsedDirRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 给定一条候选 dim 的方向、投影区间、以及中点/长度，返回它应当使用的 `dim_times`（1-based）。
    ///
    /// 算法（对应 PML `isobran` 里 `dimUsedDirs.unique() + isoUsedDir` 的效果）：
    /// 1. 在已有 entries 里找所有与 `direction` 近乎同向（cos 相似 ≥ 阈值或相反）的条目；
    /// 2. 在这些条目里找距离区间 `[dis_min, dis_max]` 与当前 dim `[q_min, q_max]` 重叠的；
    /// 3. 进一步要求两条 dim 的中点在空间上邻近（距离 < 动态阈值），排除不同段管道误判；
    /// 4. 取所有冲突条目中最大的 `dim_times`，当前 dim 的 `dim_times = max + 1`；
    /// 5. 若无冲突，`dim_times = 1`。
    pub fn next_dim_times(
        &self,
        direction: Vec3,
        dis_min: f32,
        dis_max: f32,
        mid: Vec3,
        length: f32,
    ) -> u32 {
        let mut max_conflict_times: u32 = 0;
        let dir_n = direction.normalize_or_zero();
        if dir_n.length_squared() < 1e-9 {
            return 1;
        }
        let (q_min, q_max) = sorted_range(dis_min, dis_max);
        let q_threshold = (length * SPATIAL_PROXIMITY_FACTOR).max(SPATIAL_PROXIMITY_FLOOR_MM);
        for entry in &self.entries {
            let entry_n = entry.direction.normalize_or_zero();
            if entry_n.length_squared() < 1e-9 {
                continue;
            }
            let dot_abs = dir_n.dot(entry_n).abs();
            if dot_abs < DIRECTION_ALIGN_COS_THRESHOLD {
                continue;
            }
            let (e_min, e_max) = sorted_range(entry.dis_min, entry.dis_max);
            if !ranges_overlap(q_min, q_max, e_min, e_max, DISTANCE_OVERLAP_TOLERANCE_MM) {
                continue;
            }
            let e_threshold =
                (entry.length * SPATIAL_PROXIMITY_FACTOR).max(SPATIAL_PROXIMITY_FLOOR_MM);
            let proximity_threshold = q_threshold.max(e_threshold);
            if (mid - entry.mid).length() > proximity_threshold {
                // 空间上不邻近：即便方向相同，也属于不同管段的独立尺寸，不冲突
                continue;
            }
            if entry.dim_times > max_conflict_times {
                max_conflict_times = entry.dim_times;
            }
        }
        max_conflict_times + 1
    }

    pub fn record(
        &mut self,
        direction: Vec3,
        dis_min: f32,
        dis_max: f32,
        kind: impl Into<String>,
        dim_times: u32,
        mid: Vec3,
        length: f32,
    ) {
        self.entries.push(UsedDirEntry {
            direction: direction.normalize_or_zero(),
            dis_min,
            dis_max,
            kind: kind.into(),
            dim_times,
            mid,
            length,
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn sorted_range(a: f32, b: f32) -> (f32, f32) {
    if a <= b { (a, b) } else { (b, a) }
}
fn ranges_overlap(a_min: f32, a_max: f32, b_min: f32, b_max: f32, tol: f32) -> bool {
    // 区间 [a_min, a_max] 和 [b_min, b_max] 之间若最大起点 < 最小终点 + tol 则视为重叠。
    let start = a_min.max(b_min);
    let end = a_max.min(b_max);
    end - start + tol >= 0.0 && end >= start - tol
}

/// 批量求解 linear dim 序列，自动分配 lane。
///
/// 对应 PML `isobran.putIntoIsoLine(!dim, !dimdirs, !chardirs, ...)` +
/// `isoUsedDir.appendAndDedup` 的组合：遍历输入，每条 dim 先用 `compute_linear_dim_layout`
/// 得到初始 direction，然后查询 `UsedDirRegistry` 判定是否已有同方向同距离区间的 dim，若有
/// 则把 `dim_times` 递增一层重新求解，确保 offset 阶梯化。
pub fn solve_linear_dim_series(
    inputs: &[SegmentInput],
    base_context: &BranchContext,
    params: &IsoParams,
) -> (Vec<PlacedLinearDim>, UsedDirRegistry) {
    let mut registry = UsedDirRegistry::new();
    let mut out = Vec::with_capacity(inputs.len());

    for input in inputs {
        let probe_context = BranchContext {
            dim_times: 1,
            ..base_context.clone()
        };
        let probe = compute_linear_dim_layout(input, &probe_context, params);
        let dir = Vec3::from_array(probe.direction);
        let mid = (input.start + input.end) * 0.5;
        let length = input.start.distance(input.end);
        let dis_start = input.start.length();
        let dis_end = input.end.length();

        let dim_times = registry.next_dim_times(dir, dis_start, dis_end, mid, length);

        let final_placed = if dim_times == 1 {
            probe
        } else {
            let context = BranchContext {
                dim_times,
                ..base_context.clone()
            };
            compute_linear_dim_layout(input, &context, params)
        };

        registry.record(
            dir,
            dis_start,
            dis_end,
            input.kind.clone(),
            dim_times,
            mid,
            length,
        );
        out.push(final_placed);
    }

    (out, registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mbd::iso_params::{BranchContext, IsoParams, SegmentInput};

    fn test_seg(id: &str, start: Vec3, end: Vec3, od: f32) -> SegmentInput {
        SegmentInput {
            id: id.to_string(),
            kind: "segment".to_string(),
            start,
            end,
            pipe_dir: (end - start).normalize_or_zero(),
            od,
            text: "0".to_string(),
            isoline_index: None,
        }
    }

    fn record_at(reg: &mut UsedDirRegistry, dir: Vec3, mid: Vec3, length: f32, times: u32) {
        reg.record(dir, 0.0, length, "segment", times, mid, length);
    }

    #[test]
    fn registry_returns_1_when_empty() {
        let reg = UsedDirRegistry::new();
        assert_eq!(
            reg.next_dim_times(Vec3::Z, 0.0, 100.0, Vec3::ZERO, 100.0),
            1
        );
    }

    #[test]
    fn registry_conflict_same_position_bumps_times() {
        let mut reg = UsedDirRegistry::new();
        let mid = Vec3::new(1000.0, 0.0, 0.0);
        record_at(&mut reg, Vec3::Z, mid, 100.0, 1);
        assert_eq!(
            reg.next_dim_times(Vec3::Z, 0.0, 100.0, mid, 100.0),
            2,
            "same direction + same position should bump"
        );
        record_at(&mut reg, Vec3::Z, mid, 100.0, 2);
        assert_eq!(reg.next_dim_times(Vec3::Z, 0.0, 100.0, mid, 100.0), 3);
    }

    #[test]
    fn registry_conflict_antiparallel_at_same_position_is_also_conflict() {
        let mut reg = UsedDirRegistry::new();
        let mid = Vec3::new(500.0, 0.0, 0.0);
        record_at(&mut reg, Vec3::Z, mid, 100.0, 1);
        assert_eq!(reg.next_dim_times(-Vec3::Z, 0.0, 100.0, mid, 100.0), 2);
    }

    #[test]
    fn registry_different_direction_not_conflict() {
        let mut reg = UsedDirRegistry::new();
        let mid = Vec3::new(100.0, 100.0, 0.0);
        record_at(&mut reg, Vec3::Z, mid, 100.0, 1);
        assert_eq!(reg.next_dim_times(Vec3::X, 0.0, 100.0, mid, 100.0), 1);
    }

    #[test]
    fn registry_same_direction_far_away_not_conflict() {
        // 不同管段的同向 dim，只要中点空间距离足够远，就不算冲突
        let mut reg = UsedDirRegistry::new();
        record_at(&mut reg, Vec3::Z, Vec3::new(0.0, 0.0, 0.0), 1000.0, 1);
        // 另一个 mid 距离 10m（≥ 0.25*长度 = 250mm 阈值，也超过 SPATIAL_PROXIMITY_FLOOR 300mm）
        assert_eq!(
            reg.next_dim_times(Vec3::Z, 0.0, 1000.0, Vec3::new(10000.0, 0.0, 0.0), 1000.0,),
            1,
        );
    }

    #[test]
    fn series_stagger_two_same_dir_dims() {
        // 两条重叠的水平长管 dim，应当得到 offset=od 和 offset=od+1.2*cheight
        let segs = vec![
            test_seg(
                "dim:seg:0",
                Vec3::new(14706.1, -16381.3, -1546.09),
                Vec3::new(-229.0, 0.0, 0.0),
                229.0,
            ),
            test_seg(
                "dim:chain:0",
                Vec3::new(14706.1, -16381.3, -1546.09),
                Vec3::new(-229.0, 0.0, 0.0),
                229.0,
            ),
        ];
        let ctx = BranchContext {
            branch_refno: "24381_145712".to_string(),
            bran_volume_center: Vec3::new(7238.55, -8190.65, -773.045),
            dim_times: 1,
        };
        let params = IsoParams {
            cheight: 100.0,
            ..IsoParams::default()
        };
        let (placed, reg) = solve_linear_dim_series(&segs, &ctx, &params);
        assert_eq!(placed.len(), 2);
        assert!(
            (placed[0].offset - 229.0).abs() < 1e-3,
            "first dim offset should be od=229, got {}",
            placed[0].offset
        );
        assert!(
            (placed[1].offset - (229.0 + 120.0)).abs() < 1e-3,
            "second dim offset should be od+1.2*cheight=349, got {}",
            placed[1].offset
        );
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn series_no_stagger_for_orthogonal_dirs() {
        // 两条 dim 方向差 90°（Z vs 水平），不应相互推下一 lane
        let segs = vec![
            test_seg(
                "horiz",
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(10000.0, 0.0, 0.0),
                229.0,
            ),
            test_seg(
                "vert",
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 10000.0),
                229.0,
            ),
        ];
        let ctx = BranchContext::for_test("B");
        let params = IsoParams::default();
        let (placed, _) = solve_linear_dim_series(&segs, &ctx, &params);
        assert_eq!(placed.len(), 2);
        assert!((placed[0].offset - 229.0).abs() < 1e-3);
        assert!((placed[1].offset - 229.0).abs() < 1e-3);
    }

    #[test]
    fn angle_between_directions_validates_threshold() {
        // 保留一个 sanity 测试：20° 内视为同方向
        let dir_a = Vec3::Z;
        let dir_b = Vec3::new(0.0, 0.2, 1.0).normalize();
        let dot = dir_a.dot(dir_b);
        assert!(dot >= DIRECTION_ALIGN_COS_THRESHOLD);
        let _ = angle_deg(dir_a, dir_b); // 只是确认 angle_deg 可用
    }
}
