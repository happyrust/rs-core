//! 已用方向记录 — 移植自 PML `isoUsedDir`。
//!
//! 管道标注在放置后会注册一条"已用方向记录"，包含方向、沿管段轴的距离区间、
//! 以及角度和半径约束。后续标注在选择偏移层级（`dimtimes`）时检查已用方向，
//! 避免与已有标注重叠。
//!
//! 参考文件：
//! - `rs-core/MBD/markpipe/object/isoUsedDir.pmlobj`
//! - `rs-core/MBD/markpipe/function/isoFindIsoUsedDir.pmlfnc`
//! - `rs-core/MBD/markpipe/object/isoDim.pmlobj` → `draw` 方法中的 `dimtimes` 计算

use super::primitive::Vec3V2;

/// 已用方向记录。
///
/// 每次标注放置后，记录该标注占用的方向和沿管段方向的距离区间。
/// 后续标注检查是否与已有记录冲突。
#[derive(Debug, Clone)]
pub struct IsoUsedDir {
    /// 标识名（如 "ISODIM", "ISODIM-1", "SlopeDim-1"）。
    pub name: String,
    /// 标注偏移方向。
    pub direction: Vec3V2,
    /// 沿管段方向的最小距离（从管段起点测量）。
    pub min_dis: f32,
    /// 沿管段方向的最大距离。
    pub max_dis: f32,
    /// 用途描述（如 "MainDim", "SlopeDim-1", "Handle"）。
    pub kind: String,
    /// 角度容差（度），默认 5°。
    pub angle_tolerance: f32,
    /// 最小占用半径，默认 0。
    pub min_radius: f32,
    /// 最大占用半径，默认 10000。
    pub max_radius: f32,
}

impl IsoUsedDir {
    pub fn new(name: &str, direction: Vec3V2, min_dis: f32, max_dis: f32, kind: &str) -> Self {
        Self {
            name: name.to_string(),
            direction,
            min_dis,
            max_dis,
            kind: kind.to_string(),
            angle_tolerance: 5.0,
            min_radius: 0.0,
            max_radius: 10000.0,
        }
    }
}

/// 已用方向注册表。
///
/// 在标注流程中，维护一个 `UsedDirRegistry`，每次标注放置后 `register` 记录，
/// 后续标注通过 `count_overlaps` 确定需要偏移几层。
#[derive(Debug, Clone, Default)]
pub struct UsedDirRegistry {
    pub(crate) dirs: Vec<IsoUsedDir>,
}

impl UsedDirRegistry {
    pub fn new() -> Self {
        Self { dirs: Vec::new() }
    }

    /// 注册一条已用方向。
    pub fn register(&mut self, used_dir: IsoUsedDir) {
        self.dirs.push(used_dir);
    }

    /// 移植自 `isoFindIsoUsedDir`：按名称查找。
    pub fn find_by_name(&self, name: &str) -> Option<&IsoUsedDir> {
        self.dirs.iter().find(|d| d.name == name)
    }

    /// 计算给定方向和距离区间与已有记录的重叠数。
    ///
    /// 返回值作为 `dimtimes` 的增量：0 表示无冲突，N 表示需要偏移 N 层。
    /// 两条记录"冲突"的判定：
    /// 1. 方向夹角 < `angle_tolerance`（或 > 180° - tolerance，即同向或反向）
    /// 2. 距离区间有交集
    pub fn count_overlaps(
        &self,
        direction: Vec3V2,
        min_dis: f32,
        max_dis: f32,
    ) -> u32 {
        let dir = glam::Vec3::new(direction[0], direction[1], direction[2]);
        let mut count = 0u32;

        for used in &self.dirs {
            let used_dir = glam::Vec3::new(
                used.direction[0],
                used.direction[1],
                used.direction[2],
            );

            let angle = dir.angle_between(used_dir).to_degrees();
            let is_parallel = angle < used.angle_tolerance
                || (180.0 - angle) < used.angle_tolerance;

            if !is_parallel {
                continue;
            }

            let has_overlap = min_dis < used.max_dis && max_dis > used.min_dis;

            if has_overlap {
                count += 1;
            }
        }

        count
    }

    /// 基于已用方向计算标注应该偏移的层数（dimtimes）。
    ///
    /// `base_dimtimes`：基础层数（通常为 1）。
    /// 返回值 = base_dimtimes + 重叠数。
    pub fn compute_dimtimes(
        &self,
        direction: Vec3V2,
        min_dis: f32,
        max_dis: f32,
        base_dimtimes: u32,
    ) -> u32 {
        base_dimtimes + self.count_overlaps(direction, min_dis, max_dis)
    }

    pub fn len(&self) -> usize {
        self.dirs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: Vec3V2 = [1.0, 0.0, 0.0];
    const N: Vec3V2 = [0.0, 1.0, 0.0];
    const U: Vec3V2 = [0.0, 0.0, 1.0];
    const W: Vec3V2 = [-1.0, 0.0, 0.0];

    #[test]
    fn empty_registry_no_overlaps() {
        let reg = UsedDirRegistry::new();
        assert_eq!(reg.count_overlaps(N, 0.0, 100.0), 0);
    }

    #[test]
    fn same_direction_same_range_conflicts() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM", N, 0.0, 100.0, "MainDim"));
        assert_eq!(reg.count_overlaps(N, 50.0, 150.0), 1);
    }

    #[test]
    fn opposite_direction_also_conflicts() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM", N, 0.0, 100.0, "MainDim"));
        let south = [0.0, -1.0, 0.0];
        assert_eq!(
            reg.count_overlaps(south, 50.0, 150.0),
            1,
            "opposite direction within angle tolerance should conflict"
        );
    }

    #[test]
    fn perpendicular_direction_no_conflict() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM", N, 0.0, 100.0, "MainDim"));
        assert_eq!(reg.count_overlaps(E, 50.0, 150.0), 0);
    }

    #[test]
    fn non_overlapping_range_no_conflict() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM", N, 0.0, 100.0, "MainDim"));
        assert_eq!(reg.count_overlaps(N, 200.0, 300.0), 0);
    }

    #[test]
    fn multiple_overlaps_counted() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM-1", N, 0.0, 100.0, "MainDim"));
        reg.register(IsoUsedDir::new("ISODIM-2", N, 50.0, 150.0, "SlopeDim-1"));
        assert_eq!(reg.count_overlaps(N, 60.0, 90.0), 2);
    }

    #[test]
    fn compute_dimtimes_adds_base() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM", N, 0.0, 100.0, "MainDim"));
        assert_eq!(reg.compute_dimtimes(N, 50.0, 150.0, 1), 2);
    }

    #[test]
    fn find_by_name_returns_correct() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("DIM-A", N, 0.0, 100.0, "MainDim"));
        reg.register(IsoUsedDir::new("DIM-B", E, 0.0, 50.0, "SlopeDim"));
        assert!(reg.find_by_name("DIM-A").is_some());
        assert!(reg.find_by_name("DIM-B").is_some());
        assert!(reg.find_by_name("DIM-C").is_none());
    }

    #[test]
    fn near_parallel_within_tolerance_conflicts() {
        let mut reg = UsedDirRegistry::new();
        reg.register(IsoUsedDir::new("ISODIM", N, 0.0, 100.0, "MainDim"));
        let nearly_north = [0.01, 1.0, 0.0];
        assert_eq!(
            reg.count_overlaps(nearly_north, 50.0, 80.0),
            1,
            "near-parallel direction within tolerance should conflict"
        );
    }
}
