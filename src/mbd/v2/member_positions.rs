//! 管件位置收集与处理 — 移植自 PML `isoDim.addmem`、`considerTee`、`possunique`。
//!
//! 管道标注的尺寸点位来自管段上各个管件（元件）的位置。
//! PML 的 `isoDim` 对象负责：
//! 1. 按管件类型（TEE, PCOM, INST, ATTA, WELD 等）投影位置到管段轴线
//! 2. 处理 TEE/OLET 的端口位置补充（`considerTee`）
//! 3. 按投影距离排序、去重（`possunique`）
//!
//! 参考文件：
//! - `rs-core/MBD/markpipe/object/isoDim.pmlobj` → `addmem`、`considerTee`、`possunique`

use super::primitive::Vec3V2;

/// 管段上的标注点位，按沿管段方向的投影距离排序。
#[derive(Debug, Clone)]
pub struct DimPositions {
    /// 管段方向（单位向量）。
    pub pipedir: Vec3V2,
    /// 管段轴线参考起点。
    pub line_start: Vec3V2,
    /// 排序后的点位列表。
    pub positions: Vec<Vec3V2>,
    /// 各点位沿 pipedir 方向的投影距离（与 positions 一一对应）。
    pub distances: Vec<f32>,
}

impl DimPositions {
    pub fn new(pipedir: Vec3V2, line_start: Vec3V2) -> Self {
        Self {
            pipedir,
            line_start,
            positions: Vec::new(),
            distances: Vec::new(),
        }
    }

    /// 移植自 `isoDim.addpos`：插入一个点位，保持按投影距离升序。
    pub fn add_pos(&mut self, pos: Vec3V2) {
        let dis = projected_distance(self.line_start, self.pipedir, pos);

        let insert_idx = self.distances
            .iter()
            .position(|&d| d > dis)
            .unwrap_or(self.distances.len());

        self.positions.insert(insert_idx, pos);
        self.distances.insert(insert_idx, dis);
    }

    /// 移植自 `isoDim.possunique`：去除投影距离过近（< threshold）的相邻点。
    pub fn dedup(&mut self, threshold: f32) {
        if self.positions.len() < 2 {
            return;
        }

        let mut to_remove = Vec::new();
        for i in (1..self.positions.len()).rev() {
            let dist = (self.distances[i] - self.distances[i - 1]).abs();
            if dist < threshold {
                to_remove.push(i);
            }
        }

        for idx in to_remove {
            self.positions.remove(idx);
            self.distances.remove(idx);
        }
    }

    /// 总跨度：最后一个点到第一个点的投影距离。
    pub fn span(&self) -> f32 {
        if self.distances.len() < 2 {
            return 0.0;
        }
        self.distances.last().unwrap_or(&0.0) - self.distances.first().unwrap_or(&0.0)
    }

    /// 返回排序后的点位数量。
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// TEE 端口补充参数。
pub struct TeePortInfo {
    /// TEE 元件的连接端口索引（arrive 或 leave，非 branch 端口 3）。
    pub port_index: u8,
    /// TEE 端口的世界坐标。
    pub port_position: Vec3V2,
}

/// 移植自 `isoDim.considerTee`：为管段首尾的 TEE/OLET 补充端口位置。
///
/// PML 逻辑：
/// - 如果管段的第一个元件是 TEE 且其 leave 端口不是 3（分支端口），
///   则补充另一个主管端口位置
/// - 如果管段的最后一个元件是 TEE 且其 arrive 端口不是 3，同理
pub fn consider_tee(
    dim_positions: &mut DimPositions,
    first_tee: Option<TeePortInfo>,
    last_tee: Option<TeePortInfo>,
) {
    if let Some(tee) = first_tee {
        if tee.port_index != 3 {
            dim_positions.add_pos(tee.port_position);
        }
    }
    if let Some(tee) = last_tee {
        if tee.port_index != 3 {
            dim_positions.add_pos(tee.port_position);
        }
    }
}

fn projected_distance(origin: Vec3V2, dir: Vec3V2, point: Vec3V2) -> f32 {
    let dx = point[0] - origin[0];
    let dy = point[1] - origin[1];
    let dz = point[2] - origin[2];
    dx * dir[0] + dy * dir[1] + dz * dir[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_pos_maintains_order() {
        let mut dp = DimPositions::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        dp.add_pos([300.0, 0.0, 0.0]);
        dp.add_pos([100.0, 0.0, 0.0]);
        dp.add_pos([200.0, 0.0, 0.0]);

        assert_eq!(dp.len(), 3);
        assert!((dp.distances[0] - 100.0).abs() < 0.01);
        assert!((dp.distances[1] - 200.0).abs() < 0.01);
        assert!((dp.distances[2] - 300.0).abs() < 0.01);
    }

    #[test]
    fn dedup_removes_close_points() {
        let mut dp = DimPositions::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        dp.add_pos([0.0, 0.0, 0.0]);
        dp.add_pos([0.005, 0.0, 0.0]);
        dp.add_pos([100.0, 0.0, 0.0]);

        dp.dedup(0.01);
        assert_eq!(dp.len(), 2, "close points should be merged");
        assert!((dp.distances[1] - 100.0).abs() < 0.01);
    }

    #[test]
    fn span_calculation() {
        let mut dp = DimPositions::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        dp.add_pos([50.0, 0.0, 0.0]);
        dp.add_pos([150.0, 0.0, 0.0]);
        assert!((dp.span() - 100.0).abs() < 0.01);
    }

    #[test]
    fn consider_tee_adds_non_branch_port() {
        let mut dp = DimPositions::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        dp.add_pos([0.0, 0.0, 0.0]);
        dp.add_pos([100.0, 0.0, 0.0]);

        consider_tee(
            &mut dp,
            Some(TeePortInfo {
                port_index: 1,
                port_position: [-50.0, 0.0, 0.0],
            }),
            None,
        );
        assert_eq!(dp.len(), 3);
        assert!(dp.distances[0] < 0.0, "TEE port should be before start");
    }

    #[test]
    fn consider_tee_skips_branch_port() {
        let mut dp = DimPositions::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        dp.add_pos([0.0, 0.0, 0.0]);

        consider_tee(
            &mut dp,
            Some(TeePortInfo {
                port_index: 3,
                port_position: [-50.0, 0.0, 0.0],
            }),
            None,
        );
        assert_eq!(dp.len(), 1, "branch port 3 should be skipped");
    }

    #[test]
    fn projected_distance_works() {
        let d = projected_distance(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [50.0, 30.0, 20.0],
        );
        assert!((d - 50.0).abs() < 0.01);
    }
}
