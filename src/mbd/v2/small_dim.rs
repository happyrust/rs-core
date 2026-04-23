//! 小尺寸拆分与字高自适应 — 复刻 PDMS `lindim.sepSmallDim` + `changeCheightAuto`。
//!
//! 当链式尺寸中某一段长度不足以容纳标注文字时，本模块决定：
//! 1. 缩小字高（`changeCheightAuto`）
//! 2. 错层放置（`sepSmallDim`，`level` 递增）
//! 3. 正常累积（文字可容纳时合并连续段为一个 row）
//!
//! 输出 `Vec<DimRow>`，每个 row 对应一个（或连续多个）段的最终尺寸标注参数。

use super::primitive::Vec3V2;
use super::text_measurement::{format_dim_value, mbd_text_len};

/// 小尺寸求解器的输入。
#[derive(Debug, Clone)]
pub struct SmallDimInput {
    /// 链式尺寸的各个测量点（≥2 个点，相邻两点形成一段）。
    pub points: Vec<Vec3V2>,
    /// 标注文字方向（`lindim.ori.xdir()`），投影段长到此方向上。
    pub xdir: Vec3V2,
    /// 标注文字的上方向（`lindim.ori.ydir()`），用于错层偏移。
    pub ydir: Vec3V2,
    /// 标注放置位置（dim line 的基准点）。
    pub pos: Vec3V2,
    /// 基准字高（mm）。
    pub cheight: f32,
    /// 是否开启小尺寸拆分（错层）。
    pub sep_small_dim: bool,
    /// 是否开启字高自适应。
    pub change_cheight_auto: bool,
    /// 字高缩小比例下限（默认 0.5，即最低可缩到原字高的 50%）。
    pub change_cheight_auto_bili: f32,
}

impl Default for SmallDimInput {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            xdir: [1.0, 0.0, 0.0],
            ydir: [0.0, 1.0, 0.0],
            pos: [0.0, 0.0, 0.0],
            cheight: 2.5,
            sep_small_dim: true,
            change_cheight_auto: true,
            change_cheight_auto_bili: 0.5,
        }
    }
}

/// 小尺寸求解器的一行输出。
#[derive(Debug, Clone, PartialEq)]
pub struct DimRow {
    /// 该行覆盖的点（≥2 个，相邻两点是一段）。
    pub points: Vec<Vec3V2>,
    /// 标注放置位置（已含错层偏移）。
    pub pos: Vec3V2,
    /// 该行实际使用的字高（mm）。
    pub cheight: f32,
    /// 错层层号：0 = 基础层，>0 = 向上/下偏移。
    pub level: u16,
    /// 每段对应的标注文字。
    pub texts: Vec<String>,
}

/// 小尺寸求解的完整输出。
#[derive(Debug, Clone)]
pub struct SmallDimResult {
    pub rows: Vec<DimRow>,
}

/// 计算点到方向的投影距离（PDMS `!!suidirdis` 等价）。
fn projected_distance(p1: Vec3V2, dir: Vec3V2, p2: Vec3V2) -> f32 {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];
    let dz = p2[2] - p1[2];
    dx * dir[0] + dy * dir[1] + dz * dir[2]
}

/// PDMS `getgoodcheight` 等价：给定一组点和方向，算出使文字恰好占满
/// `bili` 比例的"合适字高"。
///
/// `cheight = (投影总跨度 × bili) / (各段文字 em 宽度之和)`
fn get_good_cheight(points: &[Vec3V2], xdir: Vec3V2, bili: f32) -> f32 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut em_sum = 0.0_f32;
    for i in 0..points.len() - 1 {
        let dis = projected_distance(points[i], xdir, points[i + 1]).abs();
        let text = format_dim_value(dis);
        em_sum += mbd_text_len(&text);
    }
    if em_sum <= 0.0 {
        return 0.0;
    }
    let dim_len = projected_distance(points[0], xdir, *points.last().unwrap()).abs();
    dim_len * bili / em_sum
}

/// 求向量 a 和向量 b 的点积。
fn dot3(a: Vec3V2, b: Vec3V2) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// 向量加法：a + b * scale。
fn vec3_add_scaled(a: Vec3V2, b: Vec3V2, scale: f32) -> Vec3V2 {
    [
        a[0] + b[0] * scale,
        a[1] + b[1] * scale,
        a[2] + b[2] * scale,
    ]
}

/// 求解小尺寸拆分。
///
/// 核心逻辑复刻自 PDMS `lindim.sepSmallDim()` + `changeCheightAuto()`。
pub fn solve_small_dims(input: &SmallDimInput) -> SmallDimResult {
    let n = input.points.len();
    if n < 2 {
        return SmallDimResult { rows: Vec::new() };
    }

    if !input.sep_small_dim && !input.change_cheight_auto {
        let mut texts = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let dis = projected_distance(input.points[i], input.xdir, input.points[i + 1]).abs();
            texts.push(format_dim_value(dis));
        }
        return SmallDimResult {
            rows: vec![DimRow {
                points: input.points.clone(),
                pos: input.pos,
                cheight: input.cheight,
                level: 0,
                texts,
            }],
        };
    }

    let mut rows: Vec<DimRow> = Vec::new();
    let mut current_level: u16 = 0;

    let mut acc_points: Vec<Vec3V2> = Vec::new();
    let mut acc_texts: Vec<String> = Vec::new();

    for seg_idx in 0..n - 1 {
        let p_start = input.points[seg_idx];
        let p_end = input.points[seg_idx + 1];
        let dis = projected_distance(p_start, input.xdir, p_end).abs();
        let text = format_dim_value(dis);
        let text_em = mbd_text_len(&text);
        let text_width = text_em * input.cheight;

        if text_width <= dis {
            if acc_points.is_empty() {
                acc_points.push(p_start);
            }
            acc_points.push(p_end);
            acc_texts.push(text);
        } else {
            if acc_points.len() >= 2 {
                rows.push(DimRow {
                    points: std::mem::take(&mut acc_points),
                    pos: input.pos,
                    cheight: input.cheight,
                    level: 0,
                    texts: std::mem::take(&mut acc_texts),
                });
            } else {
                acc_points.clear();
                acc_texts.clear();
            }

            let temp_points = vec![p_start, p_end];

            if input.change_cheight_auto {
                let good_cheight = get_good_cheight(&temp_points, input.xdir, 0.8);
                let min_cheight = input.cheight * input.change_cheight_auto_bili;
                let temp_cheight = good_cheight.max(min_cheight);
                let real_width = text_em * temp_cheight;

                if real_width <= dis {
                    rows.push(DimRow {
                        points: temp_points,
                        pos: input.pos,
                        cheight: temp_cheight,
                        level: 0,
                        texts: vec![text],
                    });
                    continue;
                }
            }

            if input.sep_small_dim {
                current_level += 1;
                let mid = [
                    (p_start[0] + p_end[0]) * 0.5,
                    (p_start[1] + p_end[1]) * 0.5,
                    (p_start[2] + p_end[2]) * 0.5,
                ];
                let to_pos = [
                    input.pos[0] - mid[0],
                    input.pos[1] - mid[1],
                    input.pos[2] - mid[2],
                ];
                let sign = if dot3(to_pos, input.ydir) >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let offset = sign * input.cheight * 1.2 * current_level as f32;
                let row_pos = vec3_add_scaled(input.pos, input.ydir, offset);

                rows.push(DimRow {
                    points: temp_points,
                    pos: row_pos,
                    cheight: input.cheight,
                    level: current_level,
                    texts: vec![text],
                });
            } else {
                let min_cheight = input.cheight * input.change_cheight_auto_bili;
                let good_cheight = get_good_cheight(&temp_points, input.xdir, 0.8);
                let temp_cheight = good_cheight.max(min_cheight);
                rows.push(DimRow {
                    points: temp_points,
                    pos: input.pos,
                    cheight: temp_cheight,
                    level: 0,
                    texts: vec![text],
                });
            }
        }
    }

    if acc_points.len() >= 2 {
        rows.push(DimRow {
            points: acc_points,
            pos: input.pos,
            cheight: input.cheight,
            level: 0,
            texts: acc_texts,
        });
    }

    SmallDimResult { rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_straight_points(lengths: &[f32]) -> Vec<Vec3V2> {
        let mut pts = vec![[0.0, 0.0, 0.0]];
        let mut x = 0.0_f32;
        for &len in lengths {
            x += len;
            pts.push([x, 0.0, 0.0]);
        }
        pts
    }

    #[test]
    fn all_segments_fit_produces_single_row() {
        let input = SmallDimInput {
            points: make_straight_points(&[500.0, 300.0, 400.0]),
            cheight: 2.5,
            ..SmallDimInput::default()
        };
        let result = solve_small_dims(&input);
        assert_eq!(result.rows.len(), 1, "should be a single row");
        assert_eq!(result.rows[0].points.len(), 4);
        assert_eq!(result.rows[0].level, 0);
        assert_eq!(result.rows[0].texts, vec!["500", "300", "400"]);
    }

    #[test]
    fn short_segment_triggers_layer_split() {
        // cheight=2.5, "1" em width ~0.5741, text width = 0.5741*2.5 ≈ 1.435
        // segment length = 1.0 < 1.435 → should trigger split
        let input = SmallDimInput {
            points: make_straight_points(&[500.0, 1.0, 400.0]),
            cheight: 2.5,
            sep_small_dim: true,
            change_cheight_auto: false,
            ..SmallDimInput::default()
        };
        let result = solve_small_dims(&input);
        assert!(
            result.rows.len() >= 2,
            "short segment should cause split, got {} rows",
            result.rows.len()
        );
        let short_row = result.rows.iter().find(|r| r.level > 0);
        assert!(short_row.is_some(), "should have a row with level > 0");
    }

    #[test]
    fn change_cheight_auto_shrinks_text() {
        // segment length = 2.0, cheight=2.5, text "2" em=0.5762
        // text_width = 0.5762 * 2.5 = 1.4405 ≤ 2.0 → fits without shrink
        // But segment length = 1.0, text_width = 1.4405 > 1.0 → needs action
        let input = SmallDimInput {
            points: make_straight_points(&[500.0, 1.0, 400.0]),
            cheight: 2.5,
            sep_small_dim: false,
            change_cheight_auto: true,
            change_cheight_auto_bili: 0.5,
            ..SmallDimInput::default()
        };
        let result = solve_small_dims(&input);
        let shrunk_row = result.rows.iter().find(|r| r.cheight < 2.5);
        assert!(
            shrunk_row.is_some(),
            "should have a row with reduced cheight"
        );
        assert!(
            shrunk_row.unwrap().cheight >= 2.5 * 0.5,
            "cheight should not go below floor"
        );
    }

    #[test]
    fn no_split_no_auto_produces_single_row() {
        let input = SmallDimInput {
            points: make_straight_points(&[500.0, 1.0, 400.0]),
            cheight: 2.5,
            sep_small_dim: false,
            change_cheight_auto: false,
            ..SmallDimInput::default()
        };
        let result = solve_small_dims(&input);
        assert_eq!(result.rows.len(), 1, "should produce exactly one row");
        assert_eq!(result.rows[0].level, 0);
    }

    #[test]
    fn empty_input_produces_no_rows() {
        let input = SmallDimInput {
            points: vec![],
            ..SmallDimInput::default()
        };
        let result = solve_small_dims(&input);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn single_point_produces_no_rows() {
        let input = SmallDimInput {
            points: vec![[0.0, 0.0, 0.0]],
            ..SmallDimInput::default()
        };
        let result = solve_small_dims(&input);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn format_dim_value_integration() {
        assert_eq!(format_dim_value(500.0), "500");
        assert_eq!(format_dim_value(12.5), "12.5");
        assert_eq!(format_dim_value(1.23), "1.23");
    }

    #[test]
    fn get_good_cheight_basic() {
        let points = vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0]];
        let xdir = [1.0, 0.0, 0.0];
        let cheight = get_good_cheight(&points, xdir, 0.8);
        // dim_len = 100, text "100" em = mbd_text_len("100")
        let em = mbd_text_len("100");
        let expected = 100.0 * 0.8 / em;
        assert!(
            (cheight - expected).abs() < 0.01,
            "expected ~{:.2}, got {:.2}",
            expected,
            cheight
        );
    }
}
