use crate::parsed_data::{CateProfileParam, SProfileData, SannData};
use crate::prim_geo::spine::{SegmentPath, *};
use crate::prim_geo::wire;
use crate::shape::pdms_shape::{ANGLE_RAD_F64_TOL, BrepShapeTrait, VerifiedShape};
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::{BrepMathTrait, convert_to_cg_matrix4};
use crate::tool::math_tool::{quat_to_pdms_ori_str, to_pdms_ori_str};
use anyhow::anyhow;
use approx::{abs_diff_eq, abs_diff_ne};
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use cavalier_contours::core::math::bulge_from_angle;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, Polyline, seg_midpoint};
use glam::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::prim_geo::wire::polyline_to_debug_json_str;
#[cfg(feature = "occ")]
use opencascade::angle::ToAngle;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
#[cfg(feature = "truck")]
use truck_base::cgmath64::*;

///含有两边方向的，扫描体
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
pub struct SweepSolid {
    pub profile: CateProfileParam,
    pub drns: Option<DVec3>,
    pub drne: Option<DVec3>,
    pub plax: Vec3,
    pub bangle: f32,
    pub extrude_dir: DVec3,
    pub height: f32,
    pub path: SweepPath3D,
    pub lmirror: bool,
    pub spine_segments: Vec<Spine3D>, // 存储原始 Spine3D 段信息（用于变换）
    pub segment_transforms: Vec<Transform>, // 存储每段起点 POINSP 的 local transform
}

impl SweepSolid {
    #[inline]
    pub fn is_sloped(&self) -> bool {
        self.is_drns_sloped() || self.is_drne_sloped()
    }

    #[inline]
    pub fn is_drns_sloped(&self) -> bool {
        self.drns
            .map(|v| abs_diff_ne!(v.z, 1.0, epsilon = 0.001))
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_drne_sloped(&self) -> bool {
        self.drne
            .map(|v| abs_diff_ne!(v.z, 1.0, epsilon = 0.001))
            .unwrap_or(false)
    }

    //获得drns/drne的面的旋转矩阵
    pub fn get_face_mat4(&self, is_start: bool) -> DMat4 {
        let axis = if is_start {
            if self.drns.is_none() {
                return DMat4::IDENTITY;
            }
            DVec3::Z
        } else {
            if self.drne.is_none() {
                return DMat4::IDENTITY;
            }
            DVec3::NEG_Z
        };
        let dir = if is_start {
            self.drns.unwrap()
        } else {
            self.drne.unwrap()
        };
        if dir.z.abs() < 0.1 {
            return DMat4::IDENTITY;
        }
        let mut angle_x = (dir.x / dir.z).atan();
        let mut angle_y = -(dir.y / dir.z).atan();
        //这里这个角度限制，应该用 h/2 / l 去计算，这里暂时给45°
        if angle_x.abs() - 0.01 >= FRAC_PI_2 || angle_y.abs() - 0.01 >= FRAC_PI_2 {
            return DMat4::IDENTITY;
        }
        let scale = DVec3::new(1.0 / angle_x.cos().abs(), 1.0 / angle_y.cos().abs(), 1.0);
        // dbg!((dir, angle_x.to_degrees(), angle_y.to_degrees(), scale));
        let rot =
            DQuat::from_axis_angle(DVec3::Y, angle_x) * DQuat::from_axis_angle(DVec3::X, angle_y);
        DMat4::from_scale_rotation_translation(scale, rot, DVec3::ZERO)
    }
}

impl Default for SweepSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::UNKOWN,
            drns: None,
            drne: None,
            plax: Vec3::Y,
            bangle: 0.0,
            extrude_dir: DVec3::Z,
            height: 0.0,
            path: SweepPath3D::default(),
            lmirror: false,
            spine_segments: Vec::new(),
            segment_transforms: Vec::new(),
        }
    }
}

impl VerifiedShape for SweepSolid {
    fn check_valid(&self) -> bool {
        let has_path = !self.path.segments.is_empty() && self.path.length() > 1e-4;
        let has_profile = !matches!(self.profile, CateProfileParam::UNKOWN);

        !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0 && has_path && has_profile
    }
}

impl BrepShapeTrait for SweepSolid {
    fn is_reuse_unit(&self) -> bool {
        true
    }

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        // 仅对影响几何的参数取哈希：截面 + 归一化路径 + 端面倾斜/镜像，避免位置/缩放导致的缓存失效
        // bangle 不参与哈希，因为它在 Transform 中应用，不影响单位几何体
        #[derive(Serialize)]
        struct Hashable<'a> {
            profile: &'a CateProfileParam,
            path: &'a SweepPath3D,
            drns: &'a Option<DVec3>,
            drne: &'a Option<DVec3>,
            lmirror: bool,
            plax: Vec3,
        }

        let mut hasher = DefaultHasher::default();
        "SweepSolid".hash(&mut hasher);

        let target =
            if self.is_drns_sloped() || self.is_drne_sloped() || !self.path.is_single_segment() {
                Hashable {
                    profile: &self.profile,
                    path: &self.path,
                    drns: &self.drns,
                    drne: &self.drne,
                    lmirror: self.lmirror,
                    plax: self.plax,
                }
            } else {
                // 单段直线且无倾斜：只需截面与镜像标记
                Hashable {
                    profile: &self.profile,
                    path: &SweepPath3D::default(),
                    drns: &None,
                    drne: &None,
                    lmirror: self.lmirror,
                    plax: self.plax,
                }
            };

        if let Ok(bytes) = bincode::serialize(&target) {
            bytes.hash(&mut hasher);
        }

        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let mut unit = self.clone();
        if unit.path.as_single_line().is_some() && !self.is_sloped() {
            unit.extrude_dir = DVec3::Z;
            unit.path = SweepPath3D::from_line(Line3D {
                start: Default::default(),
                end: Vec3::Z * 10.0,
                is_spine: false,
            });
        }
        // 单位体不应携带原始的段变换，避免重复应用位移/缩放
        unit.segment_transforms = vec![Transform::IDENTITY];
        unit.spine_segments.clear();
        // 单位几何体是标准的，不包含 bangle
        unit.bangle = 0.0;
        Box::new(unit)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_sloped() {
            return Vec3::ONE;
        }
        if let Some(l) = self.path.as_single_line() {
            Vec3::new(1.0, 1.0, l.length() / 10.0)
        } else {
            Vec3::ONE
        }
    }

    #[inline]
    fn get_trans(&self) -> bevy_transform::prelude::Transform {
        // 使用 segment_transforms 中的第一个变换（如果存在）
        if let Some(first_transform) = self.segment_transforms.first() {
            *first_transform
        } else {
            Transform {
                rotation: Quat::IDENTITY,
                scale: self.get_scaled_vec3(),
                translation: Vec3::ZERO,
            }
        }
    }

    fn tol(&self) -> f32 {
        if let Some(aabb) = self.profile.get_bbox() {
            return 0.01 * aabb.bounding_sphere().radius.max(1.0);
        }
        0.01
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLoft(self.clone()))
    }

    /// 生成 CSG 网格
    ///
    /// 支持单段路径（Line/SpineArc）和多段路径（MultiSegment）
    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        use crate::geometry::sweep_mesh::generate_sweep_solid_mesh;
        use crate::mesh_precision::LodMeshSettings;

        let settings = LodMeshSettings::default();

        if let Some(mesh) = generate_sweep_solid_mesh(self, &settings, None) {
            Ok(crate::prim_geo::basic::CsgSharedMesh::new(mesh))
        } else {
            let path_desc = if self.path.is_single_segment() {
                match self.path.segments.first() {
                    Some(SegmentPath::Line(_)) => "单段直线",
                    Some(SegmentPath::Arc(_)) => "单段圆弧",
                    None => "空路径",
                }
            } else {
                &format!("{}段混合路径", self.path.segment_count())
            };

            Err(anyhow::anyhow!(
                "SweepSolid 的 CSG 网格生成失败。\n\
                路径类型: {}\n\
                可能原因: 不支持的截面类型或路径类型",
                path_desc
            ))
        }
    }
}

fn cal_end_face_rot(current_rot: DQuat, extru_dir: DVec3, face_dir: Option<DVec3>) -> DMat4 {
    let mut mat = DMat4::IDENTITY;
    if let Some(mut fd) = face_dir {
        let dir = current_rot.mul_vec3(extru_dir);
        //求两者之间的夹角，如果是负数，就是反方向
        let angle = dir.angle_between(fd);
        //如果超过90度，就是反方向
        if angle.abs() > std::f32::consts::FRAC_PI_2 as _ {
            fd = -fd;
        }
        // dbg!(angle);
        let dir_x = DVec3::new(dir.x, 0.0, dir.z).normalize();
        let fd_x = DVec3::new(fd.x, 0.0, fd.z).normalize();
        let angle_x = dir_x.angle_between(fd_x);
        let scale_x = 1.0 / angle_x.cos();

        let dir_y = DVec3::new(0.0, dir.y, dir.z).normalize();
        let fd_y = DVec3::new(0.0, fd.y, fd.z).normalize();
        let angle_y = dir_y.angle_between(fd_y);
        let scale_y = 1.0 / angle_y.cos();

        mat = DMat4::from_scale_rotation_translation(
            DVec3::new(scale_x, scale_y, 1.0),
            DQuat::IDENTITY,
            DVec3::ZERO,
        );
    }
    mat
}
