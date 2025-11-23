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
    pub bangle: f32,
    pub plax: Vec3,
    pub extrude_dir: DVec3,
    pub height: f32,
    pub path: SweepPath3D,
    pub lmirror: bool,
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

    #[cfg(feature = "occ")]
    ///计算SPRO的face
    /// start_vec 为起始方向
    fn gen_occ_spro_wire(&self, profile: &SProfileData) -> anyhow::Result<Wire> {
        let verts = &profile.verts;
        let len = verts.len();

        let mut offset_pt = DVec3::ZERO;
        let mut rot_mat = DMat3::IDENTITY;
        let mut beta_rot = DQuat::IDENTITY;
        let mut r_translation = DVec3::ZERO;
        let plin_pos = profile.plin_pos;
        // dbg!(&profile);
        // dbg!(&plin_pos);
        offset_pt.x = -plin_pos.x as f64;
        offset_pt.y = -plin_pos.y as f64;

        if let Some(arc) = self.path.as_single_arc() {
            let y_axis = arc.pref_axis.as_dvec3();
            let mut z_axis = self.plax.as_dvec3();
            r_translation.x = arc.radius as f64;
            if arc.clock_wise {
                z_axis = -z_axis;
            }
            if self.lmirror {
                z_axis = -z_axis;
            }
            let x_axis = y_axis.cross(z_axis).normalize();
            //旋转到期望的平面
            rot_mat = DMat3::from_cols(x_axis, y_axis, z_axis);
            beta_rot = DQuat::from_axis_angle(z_axis, self.bangle.to_radians() as f64);
        } else if let Some(d) = self.path.as_single_line() {
            if d.is_spine {
                // dbg!(self.bangle);
                beta_rot = DQuat::from_axis_angle(DVec3::Z, self.bangle.to_radians() as f64);
            }
            rot_mat = DMat3::from_quat(DQuat::from_rotation_arc(
                profile.na_axis.as_dvec3(),
                self.plax.as_dvec3(),
            ));
        }
        let mut points = vec![];
        for i in 0..len {
            let p = Vec3::new(verts[i][0], verts[i][1], profile.frads[i]);
            points.push(p);
        }
        let mut wire = wire::gen_occ_wires(&vec![points])?
            .pop()
            .ok_or(anyhow!("无法生成wire。"))?;
        let translation = DMat4::from_translation(offset_pt);
        let r_trans_mat = DMat4::from_translation(r_translation);
        //因为spine有可能是弧线，需要提前旋转面
        let beta_mat = DMat4::from_mat3(DMat3::from_quat(beta_rot));
        let local_mat = DMat4::from_mat3(rot_mat);
        // dbg!(local_mat);
        let final_mat = r_trans_mat * beta_mat * local_mat * translation;

        Ok(wire.transformed_by_gmat(&final_mat)?)
    }

    ///计算SPRO的face
    /// start_vec 为起始方向
    #[cfg(feature = "truck")]
    fn cal_spro_wire(&self, profile: &SProfileData) -> Option<truck_modeling::Wire> {
        #[cfg(feature = "truck")]
        use truck_meshalgo::prelude::*;
        #[cfg(feature = "truck")]
        use truck_modeling::{Surface, builder};

        let verts = &profile.verts;
        let len = verts.len();

        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vector3::new(0.0, 0.0, 0.0);
        let plin_pos = profile.plin_pos;
        // dbg!(&profile);
        offset_pt.x = -plin_pos.x;
        offset_pt.y = -plin_pos.y;
        if let Some(arc) = self.path.as_single_arc() {
            let y_axis = arc.pref_axis;
            let mut z_axis = self.plax;
            r_translation.x = arc.radius as f64;
            if arc.clock_wise {
                z_axis = -z_axis;
            }
            if self.lmirror {
                z_axis = -z_axis;
            }
            let x_axis = y_axis.cross(z_axis).normalize();
            //旋转到期望的平面
            rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
        } else if let Some(_d) = self.path.as_single_line() {
            rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Y));
            // dbg!(rot_mat);
            // dbg!(to_pdms_ori_str(&rot_mat));
        }

        // dbg!(&offset_pt);
        let mut points = vec![];
        for i in 0..len {
            // let p = Vec3::new(verts[i][0], verts[i][1], 0.0);
            let p = verts[i].extend(0.0);
            points.push(p);
        }
        let wire = wire::gen_wire(&points, &profile.frads).ok()?;
        // dbg!(self.bangle);
        let translation = Matrix4::from_translation(offset_pt.vector3());
        // dbg!(translation);
        let r_trans_mat = Matrix4::from_translation(r_translation);
        let m = &rot_mat;
        let local_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let m = Mat3::from_quat(beta_rot);
        let final_mat = r_trans_mat * local_mat * translation;
        // dbg!(&wire);
        let mut result_wire = builder::transformed(&wire, final_mat);
        // dbg!(result_wire.vertex_iter().collect::<Vec<_>>());
        let face = builder::try_attach_plane(&[result_wire.clone()]).ok()?;
        if let Surface::Plane(plane) = face.surface() {
            // let _s = self.plax.y as f64;
            // if plane.normal().dot(self.extrude_dir.vector3()) > 0.0 {
            //     result_wire.invert();
            // }
        }
        Some(result_wire)
    }
}

impl Default for SweepSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::UNKOWN,
            bangle: 0.0,
            plax: Vec3::Y,
            extrude_dir: DVec3::Z,
            ..Default::default()
        }
    }
}

impl VerifiedShape for SweepSolid {
    fn check_valid(&self) -> bool {
        !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0
    }
}

impl BrepShapeTrait for SweepSolid {
    fn is_reuse_unit(&self) -> bool {
        // 单段直线路径且无倾斜才可复用
        self.path.is_single_segment()
            && matches!(self.path.segments.first(), Some(SegmentPath::Line(_)))
            && !self.is_sloped()
    }

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let mut is_sann = false;
        let mut found_full_sann = false;
        let mut new_sann = None;
        let (profile_wire, top_profile_wire) = match &self.profile {
            //可能出现直接一个圆环面，就带过了，所以这里需要判断是否为360°，分成两半，然后合在一起
            CateProfileParam::SANN(p) => {
                found_full_sann = p.pangle >= 360.0;
                if found_full_sann {
                    let mut sann = p.clone();
                    sann.pangle = 180.0;
                    new_sann = Some(sann);
                }
                let w = (p.pwidth + p.dwid) as f64;
                let r = (p.pradius + p.drad) as f64;
                let r1 = r - w;
                let r2 = r;
                let origin = (p.xy + p.dxy).as_dvec2();
                let wire_btm = self
                    .gen_occ_sann_wire(origin, new_sann.as_ref().unwrap_or(p), true, r1, r2)
                    .ok();
                let wire_top = self
                    .gen_occ_sann_wire(origin, new_sann.as_ref().unwrap_or(p), false, r1, r2)
                    .ok();
                is_sann = true;
                (wire_btm, wire_top)
            }
            CateProfileParam::SPRO(p) => {
                let wire = self.gen_occ_spro_wire(p).ok();
                (wire, None)
            }
            CateProfileParam::SREC(p) => {
                let profile = p.convert_to_spro();
                let wire = self.gen_occ_spro_wire(&profile).ok();
                (wire, None)
            }
            _ => (None, None),
        };
        // dbg!(found_full_sann);
        let other_half_shape = if found_full_sann {
            let mut new_data = self.clone();
            if let CateProfileParam::SANN(sann) = &mut new_data.profile {
                sann.pangle = -180.0;
            }
            // dbg!(&new_data);
            new_data.gen_occ_shape().ok()
        } else {
            None
        };
        // dbg!(other_half_shape.is_some());
        if let Some(mut wire) = profile_wire {
            if let Some(arc) = self.path.as_single_arc() {
                let rot_angle = arc.angle;
                let rot_axis = if arc.clock_wise { -DVec3::Z } else { DVec3::Z };
                let r = wire
                    .to_face()
                    .revolve(DVec3::ZERO, rot_axis, Some(rot_angle.radians()));
                let shape = r.into_shape();
                if let Some(other_half) = other_half_shape {
                    let mut new_shape = shape.union(&other_half).shape;
                    return Ok(new_shape.into());
                }
                return Ok(shape.into_shape().into());
            } else if let Some(l) = self.path.as_single_line() {
                let mut wires = vec![];
                let mut transform_btm = self.get_face_mat4(true);
                let mut transform_top = self.get_face_mat4(false);
                transform_top =
                    DMat4::from_translation(DVec3::Z * l.length() as f64) * transform_top;
                wires.push(wire.transformed_by_gmat(&transform_btm)?);
                if let Some(mut top_wire) = top_profile_wire {
                    wires.push(top_wire.transformed_by_gmat(&transform_top)?);
                } else {
                    wires.push(wire.transformed_by_gmat(&transform_top)?);
                }
                let shape = Solid::loft(wires.iter()).into_shape();
                if let Some(other_half) = other_half_shape {
                    let mut new_shape = shape.union(&other_half).shape;
                    return Ok(new_shape.into());
                }
                return Ok(shape.into_shape().into());
            }
        }

        return Err(anyhow!("SweepSolid 生成错误"));
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = if self.is_drns_sloped() || self.is_drne_sloped() {
            bincode::serialize(&self).unwrap()
        } else if self.path.as_single_arc().is_some() {
            bincode::serialize(&self).unwrap()
        } else {
            bincode::serialize(&self.profile).unwrap()
        };
        bytes.hash(&mut hasher);
        "loft".hash(&mut hasher);

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
        match &self.profile {
            CateProfileParam::SANN(_p) => {
                return bevy_transform::prelude::Transform {
                    rotation: Quat::IDENTITY,
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                };
            }
            CateProfileParam::SPRO(_) | CateProfileParam::SREC(_) => {
                return bevy_transform::prelude::Transform {
                    rotation: Quat::IDENTITY,
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                };
            }
            _ => {}
        }

        bevy_transform::prelude::Transform::IDENTITY
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
