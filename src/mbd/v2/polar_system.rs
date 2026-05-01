//! 极坐标系统 — 移植自 PML `polarsystem.pmlobj`。
//!
//! PolarSystem 围绕管段轴线建立柱坐标系 `(dis, angle, radius)`，
//! 用于确定标注/标签的最佳放置位置和朝向。
//!
//! 柱坐标含义：
//! - **dis**：沿管段方向距起点的轴向距离（mm）
//! - **angle**：绕管段轴线的角度（0–360°）
//! - **radius**：到管段轴线的径向距离（mm）
//!
//! 参考文件：
//! - `rs-core/MBD/object/polarsystem/polarsystem.pmlobj`
//! - `rs-core/MBD/object/polarsystem/polarelement.pmlobj`
//! - `rs-core/MBD/object/polarsystem/getpolarelement.pmlobj`

use super::primitive::Vec3V2;

// ── 向量工具 ──

fn v3(a: Vec3V2) -> glam::Vec3 {
    glam::Vec3::new(a[0], a[1], a[2])
}

fn to_arr(v: glam::Vec3) -> Vec3V2 {
    [v.x, v.y, v.z]
}

fn dot(a: glam::Vec3, b: glam::Vec3) -> f32 {
    a.dot(b)
}

fn projected_distance(origin: Vec3V2, dir: Vec3V2, point: Vec3V2) -> f32 {
    let d = v3(point) - v3(origin);
    dot(d, v3(dir))
}

/// 管段轴线上的投影距离（PDMS `!!suidirdis` 等价）。
pub fn axis_distance(line_start: Vec3V2, line_dir: Vec3V2, point: Vec3V2) -> f32 {
    projected_distance(line_start, line_dir, point)
}

/// 点到直线的距离（径向距离）。
pub fn point_to_line_distance(line_start: Vec3V2, line_dir: Vec3V2, point: Vec3V2) -> f32 {
    let p = v3(point) - v3(line_start);
    let d = v3(line_dir).normalize();
    let proj = p.dot(d);
    let perp = p - d * proj;
    perp.length()
}

/// 计算点绕轴线的角度（度），相对于极坐标系的 orientation。
///
/// 使用 `orientation` 的 x 轴作为 0° 参考，y 轴作为 90° 参考。
pub fn polar_angle(
    line_start: Vec3V2,
    line_dir: Vec3V2,
    ori_xdir: Vec3V2,
    ori_ydir: Vec3V2,
    point: Vec3V2,
) -> f32 {
    let p = v3(point) - v3(line_start);
    let d = v3(line_dir).normalize();
    let proj_along_axis = p.dot(d);
    let perp = p - d * proj_along_axis;

    if perp.length() < 1e-6 {
        return 0.0;
    }

    let perp_n = perp.normalize();
    let x_comp = perp_n.dot(v3(ori_xdir).normalize());
    let y_comp = perp_n.dot(v3(ori_ydir).normalize());

    let mut angle = y_comp.atan2(x_comp).to_degrees();
    if angle < 0.0 {
        angle += 360.0;
    }
    angle
}

// ── 极坐标元素 ──

/// 已投影到柱坐标的空间占用。
///
/// 对应 PML `polarElement`：6 个 real 值描述在柱坐标系中的轴向/角度/径向范围。
#[derive(Debug, Clone, PartialEq)]
pub struct PolarElement {
    /// 沿轴向的起始距离（mm）。
    pub start_dis: f32,
    /// 沿轴向的结束距离（mm）。
    pub end_dis: f32,
    /// 绕轴的起始角度（度）。
    pub start_angle: f32,
    /// 绕轴的结束角度（度）。
    pub end_angle: f32,
    /// 径向起始距离（mm）。
    pub start_radius: f32,
    /// 径向结束距离（mm）。
    pub end_radius: f32,
    /// 是否是管段本体（管段本体不影响引线碰撞判定）。
    pub basic: bool,
    /// 调试标识。
    pub name: String,
}

impl Default for PolarElement {
    fn default() -> Self {
        Self {
            start_dis: 0.0,
            end_dis: 0.0,
            start_angle: 0.0,
            end_angle: 0.0,
            start_radius: 0.0,
            end_radius: 0.0,
            basic: false,
            name: String::new(),
        }
    }
}

/// 空间需求：标注需要的 (轴向长度, 角度, 径向高度)。
#[derive(Debug, Clone, Copy)]
pub struct SpaceNeeds {
    /// 所需轴向长度（mm）。
    pub dis: f32,
    /// 所需角度（度），通常 30°。
    pub angle: f32,
    /// 所需径向高度（mm），通常 ≈ cheight。
    pub radius: f32,
}

impl Default for SpaceNeeds {
    fn default() -> Self {
        Self {
            dis: 0.0,
            angle: 30.0,
            radius: 0.0,
        }
    }
}

/// 最佳位置搜索结果。
#[derive(Debug, Clone)]
pub struct PlacementResult {
    /// 最佳放置位置（世界坐标）。
    pub position: Vec3V2,
    /// 文字朝向（orientation 矩阵的 x/y 方向）。
    pub text_xdir: Vec3V2,
    /// 文字上方向。
    pub text_ydir: Vec3V2,
    /// 最佳轴向距离。
    pub best_dis: f32,
    /// 最佳角度。
    pub best_angle: f32,
    /// 最佳径向距离。
    pub best_radius: f32,
}

// ── 极坐标系统 ──

/// 围绕管段轴线的极坐标系统。
///
/// 管段轴线 `pos → pos + dir * limit_max_dis` 定义柱坐标系的 z 轴；
/// `ori` 的 x/y 轴定义角度参考面。
///
/// 已注册的 `PolarElement` 表示空间占用，`get_best_pos_and_ori` 在空闲区域
/// 中搜索加权最优的放置位置。
#[derive(Debug, Clone)]
pub struct PolarSystem {
    /// 管段起点。
    pub pos: Vec3V2,
    /// 管段方向（单位向量）。
    pub dir: Vec3V2,
    /// 极坐标系 orientation（x = showdir 方向, z = 管段方向）。
    pub ori_xdir: Vec3V2,
    pub ori_ydir: Vec3V2,
    /// 管段基础半径（如 OD/2）。
    pub basic_radius: f32,

    /// 轴向有效范围下限。
    pub limit_min_dis: f32,
    /// 轴向有效范围上限。
    pub limit_max_dis: f32,

    /// 水平方向（用于初始化观察角度）。
    pub hori_dir: Vec3V2,
    /// 观察方向（showdir）— 决定最佳角度。
    pub show_dir: Vec3V2,
    /// 主尺寸标注方向。
    pub main_dim_dir: Option<Vec3V2>,

    /// 已注册的空间占用元素。
    pub elements: Vec<PolarElement>,

    /// 参考方向（来自上一段/下一段管件方向）。
    pub reference_dirs: Vec<Vec3V2>,

    /// 观察角度（度，从 horidir 沿弧面旋转）。
    pub look_angle: f32,

    /// 是否忽略直线型元素（减少碰撞计算量）。
    pub ignore_line: bool,

    /// 是否是仪表管。
    pub is_inst_pipe: bool,
}

impl PolarSystem {
    /// 创建极坐标系统。
    ///
    /// 移植自 `polarsystem.polarSystem(startpos, endpos, cenpos, basicRadius, lookangle, isinstpipe)`。
    ///
    /// - `start`/`end`：管段起止点
    /// - `center`：分支包围盒中心，用于推导 horidir 和 showdir
    /// - `basic_radius`：管段基础半径（OD/2）
    /// - `look_angle`：观察角度（度）
    pub fn new(
        start: Vec3V2,
        end: Vec3V2,
        center: Vec3V2,
        basic_radius: f32,
        look_angle: f32,
        is_inst_pipe: bool,
    ) -> Self {
        let s = v3(start);
        let e = v3(end);
        let dir = (e - s).normalize();
        let limit_max_dis = s.distance(e);

        let mid = (s + e) * 0.5;
        let (hori_dir, show_dir, ori_xdir, ori_ydir) =
            compute_detail(start, to_arr(dir), center, to_arr(mid), look_angle);

        Self {
            pos: start,
            dir: to_arr(dir),
            ori_xdir,
            ori_ydir,
            basic_radius,
            limit_min_dis: 0.0,
            limit_max_dis,
            hori_dir,
            show_dir,
            main_dim_dir: None,
            elements: Vec::new(),
            reference_dirs: Vec::new(),
            look_angle,
            ignore_line: false,
            is_inst_pipe,
        }
    }

    /// 注册一个空间占用元素。
    ///
    /// 超出 `limit_min_dis..limit_max_dis` 太远的元素会被忽略。
    pub fn add(&mut self, element: PolarElement) {
        let threshold = 2.0 * self.basic_radius;
        if element.start_dis > self.limit_max_dis + threshold
            || element.end_dis < self.limit_min_dis - threshold
        {
            return;
        }

        let radius_limit = if self.is_inst_pipe { 50.0 } else { 15.0 };
        if element.start_radius > radius_limit * self.basic_radius {
            return;
        }

        self.elements.push(element);
    }

    /// 在已用空间的间隙中搜索最佳放置位置。
    ///
    /// 简化版实现：在轴向 `best_diss` 附近的候选区域中，对每个径向层级
    /// 寻找角度间隙最优的方向，按加权评分选择最优解。
    ///
    /// 返回 `Some(PlacementResult)` 或在极端情况下的默认位置。
    pub fn get_best_pos_and_ori(
        &self,
        needs: &SpaceNeeds,
        best_diss: &[f32],
        best_dirs: &[Vec3V2],
        is_dim: bool,
        _lead_line: bool,
    ) -> PlacementResult {
        let dis_ranges = self.compute_dis_ranges(best_diss, needs.dis);

        let mut best_result: Option<(f32, f32, f32, f32)> = None; // (weight, dis, angle, radius)

        for dis_range in &dis_ranges {
            let dis_mid = (dis_range[0] + dis_range[1]) * 0.5;

            let eles_in_range = self.get_dis_elements(dis_range);

            let radius_levels = self.compute_radius_levels(&eles_in_range, needs.radius);

            for &radius_level in &radius_levels {
                let min_radius = radius_level + needs.radius * 0.1;
                let max_radius = radius_level + needs.radius * 1.1;

                let radius_eles: Vec<&PolarElement> = eles_in_range
                    .iter()
                    .copied()
                    .filter(|e| !(e.start_radius > max_radius || e.end_radius < min_radius))
                    .collect();

                let (best_dir, angle_cha) =
                    self.get_dir_and_cha(&radius_eles, needs.angle, best_dirs, is_dim);

                let weight = self.weighted_weight(angle_cha, min_radius, false, needs.angle);

                let angle = polar_angle(
                    self.pos,
                    self.dir,
                    self.ori_xdir,
                    self.ori_ydir,
                    to_arr(v3(self.pos) + v3(best_dir) * 1000.0),
                );

                let mid_radius = (min_radius + max_radius) * 0.5;

                match &best_result {
                    None => {
                        best_result = Some((weight, dis_mid, angle, mid_radius));
                    }
                    Some((best_w, _, _, _)) if weight < *best_w => {
                        best_result = Some((weight, dis_mid, angle, mid_radius));
                    }
                    _ => {}
                }
            }
        }

        let (_, good_dis, good_angle, good_radius) = best_result.unwrap_or((
            f32::MAX,
            (self.limit_min_dis + self.limit_max_dis) * 0.5,
            0.0,
            self.basic_radius * 2.0,
        ));

        self.compute_result(good_dis, good_angle, good_radius, needs, is_dim)
    }

    /// 计算轴向候选范围。
    fn compute_dis_ranges(&self, best_diss: &[f32], need_dis: f32) -> Vec<[f32; 2]> {
        if best_diss.is_empty() {
            return vec![[self.limit_min_dis, self.limit_max_dis]];
        }

        best_diss
            .iter()
            .map(|&best| {
                let mut start = best - need_dis * 0.5;
                let mut end = best + need_dis * 0.5;
                if start < self.limit_min_dis {
                    let shift = self.limit_min_dis - start;
                    start += shift;
                    end += shift;
                }
                if end > self.limit_max_dis {
                    let shift = end - self.limit_max_dis;
                    start -= shift;
                    end -= shift;
                }
                start = start.max(self.limit_min_dis);
                end = end.min(self.limit_max_dis);
                [start, end]
            })
            .collect()
    }

    /// 获取轴向范围内的元素。
    fn get_dis_elements(&self, dis_range: &[f32; 2]) -> Vec<&PolarElement> {
        self.elements
            .iter()
            .filter(|e| {
                if self.ignore_line && (e.end_dis - e.start_dis) < 1.0 {
                    return false;
                }
                !(e.start_dis >= dis_range[1] || e.end_dis <= dis_range[0])
            })
            .collect()
    }

    /// 计算可用的径向层级。
    fn compute_radius_levels(&self, eles: &[&PolarElement], need_radius: f32) -> Vec<f32> {
        let mut radiuses: Vec<f32> = vec![self.basic_radius];

        let limit = if self.is_inst_pipe { 50.0 } else { 20.0 };

        for ele in eles {
            let r = ele.end_radius;
            if r > self.basic_radius && r < self.basic_radius * limit {
                radiuses.push(r);
            }
        }

        radiuses.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        radiuses.dedup_by(|a, b| (*a - *b).abs() < self.basic_radius * 0.05);

        let mut levels = Vec::new();
        for &r in &radiuses {
            let min_r = r + need_radius * 0.1;
            levels.push(min_r);
        }

        if levels.is_empty() {
            levels.push(self.basic_radius + need_radius * 0.1);
        }

        levels
    }

    /// 在角度间隙中找最佳方向。
    ///
    /// 简化自 PML `getDirAndCha`：
    /// 1. 收集元素的已用角度区间
    /// 2. 先尝试 best_dirs，如果无冲突直接返回
    /// 3. 否则找最大间隙的中间方向
    fn get_dir_and_cha(
        &self,
        eles: &[&PolarElement],
        need_angle: f32,
        best_dirs: &[Vec3V2],
        is_dim: bool,
    ) -> (Vec3V2, f32) {
        if eles.is_empty() {
            let dir = if !best_dirs.is_empty() {
                best_dirs[0]
            } else if let Some(d) = self.main_dim_dir {
                d
            } else {
                self.show_dir
            };
            return (dir, 0.0);
        }

        if is_dim {
            if let Some(d) = self.main_dim_dir {
                return (d, 0.0);
            }
        }

        let used_angles = self.get_used_angles(eles);
        let min_gap = need_angle * 0.5;

        for (i, dir) in best_dirs.iter().enumerate() {
            let try_angle = polar_angle(
                self.pos,
                self.dir,
                self.ori_xdir,
                self.ori_ydir,
                to_arr(v3(self.pos) + v3(*dir) * 1000.0),
            );

            let mut good = true;
            for used in &used_angles {
                if angle_in_range(try_angle, used[0], used[1])
                    || (try_angle - used[0]).abs() < min_gap
                    || (try_angle - used[1]).abs() < min_gap
                {
                    good = false;
                    break;
                }
            }

            if good {
                let cha = if i > 0 && best_dirs.len() >= 3 {
                    30.0
                } else {
                    0.0
                };
                return (*dir, cha);
            }
        }

        let normal_dirs = self.get_normal_dirs();
        for dir in &normal_dirs {
            let try_angle = polar_angle(
                self.pos,
                self.dir,
                self.ori_xdir,
                self.ori_ydir,
                to_arr(v3(self.pos) + v3(*dir) * 1000.0),
            );

            let mut good = true;
            for used in &used_angles {
                if angle_in_range(try_angle, used[0], used[1])
                    || (try_angle - used[0]).abs() < min_gap
                    || (try_angle - used[1]).abs() < min_gap
                {
                    good = false;
                    break;
                }
            }

            if good {
                let cha = v3(*dir)
                    .angle_between(v3(best_dirs.first().copied().unwrap_or(self.show_dir)))
                    .to_degrees();
                return (*dir, cha);
            }
        }

        let fallback_dir = best_dirs.first().copied().unwrap_or(self.show_dir);
        (fallback_dir, 360.0)
    }

    /// 收集已用角度区间。
    fn get_used_angles(&self, eles: &[&PolarElement]) -> Vec<[f32; 2]> {
        let mut angles: Vec<[f32; 2]> = Vec::new();

        for ele in eles {
            angles.push([ele.start_angle, ele.end_angle]);
        }

        angles.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));

        merge_angle_ranges(&mut angles);

        angles
    }

    /// 获取常用方向（移植自 `getnormaldirs`）。
    fn get_normal_dirs(&self) -> Vec<Vec3V2> {
        let mut dirs = Vec::new();

        if let Some(d) = self.main_dim_dir {
            dirs.push(d);
        }

        let pipe_angle_to_up = v3(self.dir).angle_between(glam::Vec3::Z).to_degrees();

        if pipe_angle_to_up < 10.0 || pipe_angle_to_up > 170.0 {
            let ortho = v3(self.show_dir)
                .cross(glam::Vec3::Z)
                .try_normalize()
                .unwrap_or(glam::Vec3::X);
            dirs.push(to_arr(ortho));
            dirs.push(to_arr(-ortho));
        } else {
            let zdir = compute_zdir(self.hori_dir, self.show_dir, self.dir);
            let temp_xdir = v3(self.show_dir).normalize();
            let temp_zdir = v3(zdir).normalize();
            let up_dir = temp_xdir.cross(temp_zdir).normalize();
            let down_dir = -up_dir;
            dirs.push(to_arr(up_dir));
            dirs.push(to_arr(down_dir));
        }

        dirs
    }

    /// 加权评分（移植自 `weightedweight`）。
    fn weighted_weight(
        &self,
        angle_cha: f32,
        min_radius: f32,
        have_lead: bool,
        need_angle: f32,
    ) -> f32 {
        let lead_coefficient = if have_lead { 1.3 } else { 1.0 };
        let scaled_cha = angle_cha * need_angle / 30.0;

        let angle_w = match scaled_cha {
            c if c <= 5.0 => 0.0,
            c if c <= 15.0 => 3.0,
            c if c < 20.0 => 5.0,
            c if c < 30.0 => 8.0,
            c if c < 45.0 => 10.0,
            c if c < 60.0 => 12.0,
            c if c < 80.0 => 15.0,
            _ => 25.0,
        };

        let basic = if self.basic_radius != 0.0 {
            self.basic_radius
        } else {
            200.0
        };
        let rt = min_radius / basic;

        let radius_w = if self.is_inst_pipe {
            match rt {
                r if r < 4.0 => 0.0,
                r if r < 6.0 => 1.0,
                r if r < 8.0 => 3.0,
                r if r < 10.0 => 5.0,
                r if r < 14.0 => 6.0,
                r if r < 20.0 => 7.0,
                r if r < 30.0 => 10.0,
                r if r < 40.0 => 15.0,
                _ => 50.0,
            }
        } else {
            match rt {
                r if r < 2.0 => 0.0,
                r if r < 3.0 => 1.0,
                r if r < 4.0 => 3.0,
                r if r < 5.0 => 5.0,
                r if r < 7.0 => 6.0,
                r if r < 10.0 => 7.0,
                r if r < 15.0 => 9.0,
                r if r < 20.0 => 11.0,
                r if r < 25.0 => 12.0,
                r if r < 30.0 => 13.0,
                r if r < 40.0 => 14.0,
                _ => 50.0,
            }
        };

        (angle_w + radius_w) * lead_coefficient
    }

    /// 从 (dis, angle, radius) 计算世界坐标位置和文字朝向。
    ///
    /// 移植自 `getresult`。
    fn compute_result(
        &self,
        good_dis: f32,
        good_angle: f32,
        good_radius: f32,
        needs: &SpaceNeeds,
        _is_dim: bool,
    ) -> PlacementResult {
        let angle_rad = good_angle.to_radians();
        let out_dir = v3(self.ori_xdir) * angle_rad.cos() + v3(self.ori_ydir) * angle_rad.sin();
        let out_dir = out_dir.normalize();

        let pipe_angle_to_up = v3(self.dir).angle_between(glam::Vec3::Z).to_degrees();

        let char_dir = if pipe_angle_to_up > 10.0 && pipe_angle_to_up < 170.0 {
            let zdir = compute_zdir(self.hori_dir, self.show_dir, self.dir);
            let best_char = v3(zdir)
                .cross(v3(self.show_dir))
                .try_normalize()
                .unwrap_or(out_dir);
            if out_dir.angle_between(best_char).to_degrees() > 90.0 {
                -out_dir
            } else {
                out_dir
            }
        } else {
            let ortho = v3(self.show_dir)
                .cross(glam::Vec3::Z)
                .try_normalize()
                .unwrap_or(out_dir);
            if out_dir.angle_between(ortho).to_degrees() > 90.0 {
                -out_dir
            } else {
                out_dir
            }
        };

        let xdir = if v3(self.dir)
            .cross(char_dir)
            .try_normalize()
            .map(|z| z.angle_between(v3(self.show_dir)).to_degrees() > 90.0)
            .unwrap_or(false)
        {
            -v3(self.dir)
        } else {
            v3(self.dir)
        };

        let good_pos = v3(self.pos)
            + v3(self.dir) * good_dis
            + out_dir * good_radius
            + char_dir * (-needs.radius * 0.5);

        PlacementResult {
            position: to_arr(good_pos),
            text_xdir: to_arr(xdir),
            text_ydir: to_arr(char_dir),
            best_dis: good_dis,
            best_angle: good_angle,
            best_radius: good_radius,
        }
    }
}

// ── 辅助函数 ──

/// 计算 horidir、showdir、ori_xdir、ori_ydir。
///
/// 移植自 `polarsystem.getDetail`。
fn compute_detail(
    start: Vec3V2,
    dir: Vec3V2,
    center: Vec3V2,
    mid: Vec3V2,
    look_angle: f32,
) -> (Vec3V2, Vec3V2, Vec3V2, Vec3V2) {
    let d = v3(dir).normalize();

    let hori_raw = d
        .cross(glam::Vec3::Z)
        .try_normalize()
        .unwrap_or_else(|| {
            let ctm = (v3(mid) - v3(center)).try_normalize().unwrap_or(glam::Vec3::X);
            ctm.cross(glam::Vec3::Z)
                .try_normalize()
                .unwrap_or(glam::Vec3::X)
        });

    let ctm_dir = (v3(mid) - v3(center)).try_normalize();
    let hori = if let Some(ctm) = ctm_dir {
        if hori_raw.angle_between(ctm).to_degrees() > 90.0 {
            -hori_raw
        } else {
            hori_raw
        }
    } else {
        hori_raw
    };

    let pipe_angle_to_up = d.angle_between(glam::Vec3::Z).to_degrees();
    let show_dir = if pipe_angle_to_up > 10.0 && pipe_angle_to_up < 170.0 {
        let temp_dir = if hori.angle_between(d).to_degrees() < 90.0 {
            d
        } else {
            -d
        };

        let temp_ori_x = hori;
        let temp_ori_z = temp_dir;
        let temp_ori_y = temp_ori_z.cross(temp_ori_x).normalize();

        let angle_rad = look_angle.to_radians();
        let sd = temp_ori_x * angle_rad.cos() + temp_ori_y * angle_rad.sin();
        sd.normalize()
    } else {
        hori
    };

    let ori_x = show_dir;
    let ori_y = d.cross(ori_x).try_normalize().unwrap_or(glam::Vec3::Y);

    (to_arr(hori), to_arr(show_dir), to_arr(ori_x), to_arr(ori_y))
}

/// 计算 z 方向（用于确定文字上方向）。
fn compute_zdir(hori_dir: Vec3V2, show_dir: Vec3V2, pipe_dir: Vec3V2) -> Vec3V2 {
    let h = v3(hori_dir);
    let s = v3(show_dir);
    let angle = h.angle_between(s).to_degrees();

    if angle < 1.0 || angle > 179.0 {
        let d = v3(pipe_dir);
        let ortho = d.cross(h).try_normalize().unwrap_or(glam::Vec3::Z);
        if ortho.angle_between(glam::Vec3::NEG_Z).to_degrees() < 90.0 {
            to_arr(-d)
        } else {
            to_arr(d)
        }
    } else {
        let zdir = h.cross(s).try_normalize().unwrap_or(glam::Vec3::Z);
        to_arr(zdir)
    }
}

/// 判断角度是否在 [start, end] 范围内（考虑跨 360° 的情况）。
fn angle_in_range(angle: f32, start: f32, end: f32) -> bool {
    if start <= end {
        angle >= start && angle <= end
    } else {
        angle >= start || angle <= end
    }
}

/// 合并重叠的角度区间。
fn merge_angle_ranges(ranges: &mut Vec<[f32; 2]>) {
    if ranges.len() < 2 {
        return;
    }

    let mut merged = Vec::new();
    let mut current = ranges[0];

    for range in ranges.iter().skip(1) {
        if range[0] <= current[1] {
            current[1] = current[1].max(range[1]);
        } else {
            merged.push(current);
            current = *range;
        }
    }
    merged.push(current);

    *ranges = merged;
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn horizontal_pipe() -> PolarSystem {
        PolarSystem::new(
            [0.0, 0.0, 0.0],
            [1000.0, 0.0, 0.0],
            [500.0, 500.0, 0.0],
            50.0,
            30.0,
            false,
        )
    }

    #[test]
    fn new_creates_valid_system() {
        let ps = horizontal_pipe();
        assert!((ps.limit_max_dis - 1000.0).abs() < 0.1);
        assert_eq!(ps.limit_min_dis, 0.0);
        assert_eq!(ps.basic_radius, 50.0);
        assert!(ps.elements.is_empty());
    }

    #[test]
    fn add_element_within_range() {
        let mut ps = horizontal_pipe();
        ps.add(PolarElement {
            start_dis: 100.0,
            end_dis: 200.0,
            start_angle: 0.0,
            end_angle: 90.0,
            start_radius: 50.0,
            end_radius: 100.0,
            basic: true,
            name: "pipe".to_string(),
        });
        assert_eq!(ps.elements.len(), 1);
    }

    #[test]
    fn add_element_out_of_range_is_ignored() {
        let mut ps = horizontal_pipe();
        ps.add(PolarElement {
            start_dis: 2000.0,
            end_dis: 3000.0,
            ..Default::default()
        });
        assert!(ps.elements.is_empty());
    }

    #[test]
    fn empty_system_returns_center_position() {
        let ps = horizontal_pipe();
        let needs = SpaceNeeds {
            dis: 100.0,
            angle: 30.0,
            radius: 50.0,
        };
        let result = ps.get_best_pos_and_ori(&needs, &[500.0], &[ps.show_dir], false, false);

        assert!(
            (result.best_dis - 500.0).abs() < 1.0,
            "empty system should place near best_dis=500, got {}",
            result.best_dis
        );
    }

    #[test]
    fn axis_distance_computes_projection() {
        let dis = axis_distance([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [500.0, 300.0, 0.0]);
        assert!((dis - 500.0).abs() < 0.01);
    }

    #[test]
    fn point_to_line_distance_computes_perpendicular() {
        let dist =
            point_to_line_distance([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [500.0, 300.0, 0.0]);
        assert!((dist - 300.0).abs() < 0.01);
    }

    #[test]
    fn weighted_weight_zero_for_close_and_aligned() {
        let ps = horizontal_pipe();
        let w = ps.weighted_weight(0.0, ps.basic_radius * 1.5, false, 30.0);
        assert!(w < 1.0, "close & aligned should have low weight, got {}", w);
    }

    #[test]
    fn weighted_weight_high_for_far_and_misaligned() {
        let ps = horizontal_pipe();
        let w = ps.weighted_weight(90.0, ps.basic_radius * 30.0, false, 30.0);
        assert!(w > 20.0, "far & misaligned should have high weight, got {}", w);
    }

    #[test]
    fn angle_in_range_normal() {
        assert!(angle_in_range(45.0, 0.0, 90.0));
        assert!(!angle_in_range(100.0, 0.0, 90.0));
    }

    #[test]
    fn angle_in_range_wrapping() {
        assert!(angle_in_range(350.0, 300.0, 30.0));
        assert!(angle_in_range(10.0, 300.0, 30.0));
        assert!(!angle_in_range(100.0, 300.0, 30.0));
    }

    #[test]
    fn merge_overlapping_ranges() {
        let mut ranges = vec![[0.0, 30.0], [20.0, 50.0], [60.0, 90.0]];
        merge_angle_ranges(&mut ranges);
        assert_eq!(ranges.len(), 2);
        assert!((ranges[0][0]).abs() < 0.01);
        assert!((ranges[0][1] - 50.0).abs() < 0.01);
        assert!((ranges[1][0] - 60.0).abs() < 0.01);
    }
}
