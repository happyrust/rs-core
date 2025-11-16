use bevy_ecs::component::Component;
use glam::{Mat3, Quat, Vec3};

use serde::{Deserialize, Serialize};

use std::f32::consts::PI;

use crate::tool::float_tool::{f32_round_3, vec3_round_3};
use crate::{RefU64, RefnoEnum};
use bevy_transform::prelude::*;

#[derive(
    Component,
    Default,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum SpineCurveType {
    #[default]
    UNKNOWN,
    CENT,
    THRU,
    LINE,
}

#[derive(
    Component,
    Debug,
    Default,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Spine3D {
    pub refno: RefnoEnum,
    pub pt0: Vec3,
    pub pt1: Vec3,
    pub thru_pt: Vec3,
    pub center_pt: Vec3,
    pub cond_pos: Vec3,
    pub radius: f32,
    pub curve_type: SpineCurveType,
    pub preferred_dir: Vec3,
}

impl Spine3D {
    //获取两端的方向，如果是直线段，就是直线的方向，如果是圆弧，就是切线的方向
    pub fn get_dir(&self, start_or_end: bool) -> Vec3 {
        match self.curve_type {
            SpineCurveType::LINE => {
                if start_or_end {
                    (self.pt1 - self.pt0).normalize_or_zero()
                } else {
                    (self.pt0 - self.pt1).normalize_or_zero()
                }
            }
            SpineCurveType::THRU => {
                let center = circum_center(self.pt0, self.pt1, self.thru_pt);
                let vec0 = self.pt0 - center;
                let vec1 = self.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();
                let dir = if start_or_end { vec0 } else { vec1 };
                let rot = Quat::from_rotation_arc(Vec3::Z, axis);
                rot.mul_vec3(dir)
            }
            SpineCurveType::CENT => {
                let center = self.center_pt;
                let vec0 = self.pt0 - center;
                let vec1 = self.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();
                let dir = if start_or_end { vec0 } else { vec1 };
                let rot = Quat::from_rotation_arc(Vec3::Z, axis);
                rot.mul_vec3(dir)
            }
            SpineCurveType::UNKNOWN => Vec3::Z,
        }
    }
}

/// 基础路径段：直线或圆弧
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum SegmentPath {
    Line(Line3D),
    Arc(Arc3D),
}

/// 扫掠路径：由一个或多个连续的路径段组成
#[derive(
    Component,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct SweepPath3D {
    /// 路径段列表，单段路径包含一个元素，多段路径包含多个元素
    pub segments: Vec<SegmentPath>,
}

impl Default for SweepPath3D {
    fn default() -> Self {
        Self {
            segments: vec![SegmentPath::Line(Line3D::default())],
        }
    }
}

impl SegmentPath {
    pub fn length(&self) -> f32 {
        match self {
            Self::Line(line) => line.length(),
            Self::Arc(arc) => arc.angle.abs() * arc.radius,
        }
    }

    pub fn start_point(&self) -> Vec3 {
        match self {
            Self::Line(line) => line.start,
            Self::Arc(arc) => arc.start_pt,
        }
    }

    pub fn end_point(&self) -> Vec3 {
        match self {
            Self::Line(line) => line.end,
            Self::Arc(arc) => {
                let rot = Quat::from_axis_angle(arc.axis, arc.angle);
                let vec = arc.start_pt - arc.center;
                arc.center + rot.mul_vec3(vec)
            }
        }
    }

    pub fn point_at(&self, t: f32) -> Vec3 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Line(line) => line.start + (line.end - line.start) * t,
            Self::Arc(arc) => {
                let angle_at_t = arc.angle * t;
                let rot = Quat::from_axis_angle(arc.axis, angle_at_t);
                let vec = arc.start_pt - arc.center;
                arc.center + rot.mul_vec3(vec)
            }
        }
    }

    pub fn tangent_at(&self, t: f32) -> Vec3 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Line(line) => line.get_dir(true),
            Self::Arc(arc) => {
                let angle_at_t = arc.angle * t;
                let rot = Quat::from_axis_angle(arc.axis, angle_at_t);
                let radial = rot.mul_vec3(arc.start_pt - arc.center);
                let tangent = arc.axis.cross(radial).normalize();
                if arc.clock_wise {
                    -tangent
                } else {
                    tangent
                }
            }
        }
    }
}

impl SweepPath3D {
    /// 创建单段直线路径
    pub fn from_line(line: Line3D) -> Self {
        Self {
            segments: vec![SegmentPath::Line(line)],
        }
    }

    /// 创建单段圆弧路径
    pub fn from_arc(arc: Arc3D) -> Self {
        Self {
            segments: vec![SegmentPath::Arc(arc)],
        }
    }

    /// 创建多段路径
    pub fn from_segments(segments: Vec<SegmentPath>) -> Self {
        Self { segments }
    }

    /// 计算路径的总长度
    pub fn length(&self) -> f32 {
        self.segments.iter().map(|seg| seg.length()).sum()
    }

    /// 获取路径的起始点
    pub fn start_point(&self) -> Vec3 {
        self.segments
            .first()
            .map(|seg| seg.start_point())
            .unwrap_or(Vec3::ZERO)
    }

    /// 获取路径的结束点
    pub fn end_point(&self) -> Vec3 {
        self.segments
            .last()
            .map(|seg| seg.end_point())
            .unwrap_or(Vec3::ZERO)
    }

    /// 是否为单段路径
    pub fn is_single_segment(&self) -> bool {
        self.segments.len() == 1
    }

    /// 获取段数
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// 如果是单段直线路径，获取直线引用
    pub fn as_single_line(&self) -> Option<&Line3D> {
        if self.is_single_segment() {
            if let Some(SegmentPath::Line(line)) = self.segments.first() {
                return Some(line);
            }
        }
        None
    }

    /// 如果是单段圆弧路径，获取圆弧引用
    pub fn as_single_arc(&self) -> Option<&Arc3D> {
        if self.is_single_segment() {
            if let Some(SegmentPath::Arc(arc)) = self.segments.first() {
                return Some(arc);
            }
        }
        None
    }

    /// 获取可变的段列表引用
    pub fn segments_mut(&mut self) -> &mut Vec<SegmentPath> {
        &mut self.segments
    }

    /// 获取路径在指定位置的切线方向
    /// t: 归一化参数 [0.0, 1.0]，0 表示起点，1 表示终点
    pub fn tangent_at(&self, t: f32) -> Vec3 {
        let t = t.clamp(0.0, 1.0);
        
        if self.segments.is_empty() {
            return Vec3::Z;
        }
        
        // 计算总长度和每段的累积长度
        let total_length = self.length();
        if total_length < 1e-6 {
            return Vec3::Z;
        }
        
        let target_length = total_length * t;
        let mut accumulated_length = 0.0;
        
        for segment in &self.segments {
            let seg_length = segment.length();
            if accumulated_length + seg_length >= target_length {
                // 目标点在当前段中
                let local_t = if seg_length > 1e-6 {
                    (target_length - accumulated_length) / seg_length
                } else {
                    0.0
                };
                return segment.tangent_at(local_t);
            }
            accumulated_length += seg_length;
        }
        
        // 如果到达这里，返回最后一段的切线
        self.segments.last().unwrap().tangent_at(1.0)
    }

    /// 验证路径的连续性
    /// 返回 (是否连续, 第一个不连续的段索引)
    pub fn validate_continuity(&self) -> (bool, Option<usize>) {
        const EPSILON: f32 = 1e-3;
        
        for i in 0..self.segments.len().saturating_sub(1) {
            let end_pt = self.segments[i].end_point();
            let start_pt = self.segments[i + 1].start_point();
            if end_pt.distance(start_pt) > EPSILON {
                return (false, Some(i));
            }
        }
        
        (true, None)
    }
}

/// `Arc3D` 结构的定义
#[derive(
    Component,
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Arc3D {
    /// 弧的中心点
    pub center: Vec3,

    /// 弧的半径
    pub radius: f32,

    /// 弧的角度 (以弧度表示)
    pub angle: f32,

    /// 弧的起始点
    pub start_pt: Vec3,

    /// 弧的绘制方向，如果顺时针则为 `true`
    pub clock_wise: bool,

    /// 弧的轴
    pub axis: Vec3,

    /// 弧的首选轴
    pub pref_axis: Vec3,
}

#[derive(
    Component,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Line3D {
    pub start: Vec3,
    pub end: Vec3,
    pub is_spine: bool,
}

impl Line3D {
    #[inline]
    pub fn length(&self) -> f32 {
        self.start.distance(self.end)
    }

    #[inline]
    pub fn get_dir(&self, start_or_end: bool) -> Vec3 {
        if start_or_end {
            (self.end - self.start).normalize_or_zero()
        } else {
            (self.start - self.end).normalize_or_zero()
        }
    }
}

impl Default for Line3D {
    fn default() -> Self {
        Self {
            start: Default::default(),
            end: Vec3::Z,
            is_spine: false,
        }
    }
}

pub fn circum_center(pt0: Vec3, pt1: Vec3, pt2: Vec3) -> Vec3 {
    let vec0 = pt1 - pt0;
    let vec1 = pt2 - pt0;
    let a2 = vec0.dot(vec0);
    let ab = vec0.dot(vec1);
    let b2 = vec1.dot(vec1);
    let det = a2 * b2 - ab * ab;
    let u = (b2 * a2 - ab * b2) / (2.0 * det);
    let v = (-ab * a2 + b2 * a2) / (2.0 * det);
    pt0 + u * vec0 + v * vec1
}

impl Spine3D {
    pub fn generate_paths(&self) -> (SweepPath3D, Transform) {
        let mut transform = Transform::IDENTITY;
        let pref_axis = self.preferred_dir.normalize();
        
        let path = match self.curve_type {
            SpineCurveType::THRU => {
                let center = circum_center(self.pt0, self.pt1, self.thru_pt);
                let vec0 = self.pt0 - self.thru_pt;
                let vec1 = self.pt1 - self.thru_pt;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                let x_axis = (self.pt0 - center).normalize();
                let d = x_axis.dot(Vec3::Z).abs();
                let ref_axis = if approx::abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                let y_axis = ref_axis.cross(x_axis).normalize();
                let z_axis = x_axis.cross(y_axis).normalize();
                transform.rotation = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
                let arc = Arc3D {
                    center: vec3_round_3(center),
                    radius: f32_round_3(center.distance(self.pt0)),
                    angle,
                    start_pt: self.pt0,
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis,
                };
                transform.translation = center;

                SweepPath3D::from_arc(arc)
            }
            SpineCurveType::CENT => {
                // CENT 类型：中心点已知
                let center = self.center_pt;
                let vec0 = self.pt0 - center;
                let vec1 = self.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                let x_axis = (self.pt0 - center).normalize();
                let d = x_axis.dot(Vec3::Z).abs();
                let ref_axis = if approx::abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                let y_axis = ref_axis.cross(x_axis).normalize();
                let z_axis = x_axis.cross(y_axis).normalize();
                transform.rotation = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
                
                let arc = Arc3D {
                    center: vec3_round_3(center),
                    radius: f32_round_3(center.distance(self.pt0)),
                    angle,
                    start_pt: self.pt0,
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis,
                };
                transform.translation = center;
                
                SweepPath3D::from_arc(arc)
            }
            SpineCurveType::LINE => {
                transform.translation = self.pt0;
                let extru = self.pt1 - self.pt0;
                let extru_dir = extru.normalize();
                let d = extru_dir.dot(pref_axis).abs();
                let ref_axis = if approx::abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else {
                    pref_axis
                };

                let p_axis = ref_axis.cross(extru_dir).normalize();
                let y_axis = extru_dir.cross(p_axis).normalize();
                transform.rotation =
                    Quat::from_mat3(&glam::f32::Mat3::from_cols(p_axis, y_axis, extru_dir));
                
                SweepPath3D::from_line(Line3D {
                    start: Default::default(),
                    end: Vec3::Z * extru.length(),
                    is_spine: true,
                })
            }
            SpineCurveType::UNKNOWN => {
                SweepPath3D::default()
            }
        };
        
        (path, transform)
    }
}
