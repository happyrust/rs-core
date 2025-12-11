use crate::parsed_data::geo_params_data::PdmsGeoParam;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use glam::{DMat4, DVec3, Mat4, Vec3};
use nom::Parser;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f64::consts::FRAC_PI_2;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use crate::mesh_precision::LodMeshSettings;
use crate::prim_geo::basic::*;
use crate::prim_geo::helper::cal_ref_axis;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::types::attmap::AttrMap;

use crate::NamedAttrMap;
#[cfg(feature = "truck")]
use truck_modeling::*;

///元件库里的LCylinder
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
pub struct LCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3, //A Axis Direction

    pub pbdi: f32,
    pub ptdi: f32,
    //diameter
    pub pdia: f32,
    pub negative: bool,
    pub centre_line_flag: bool,
}

impl Default for LCylinder {
    fn default() -> Self {
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            pbdi: -0.5,
            ptdi: 0.5,
            pdia: 1.0,
            negative: false,
            centre_line_flag: false,
        }
    }
}

impl VerifiedShape for LCylinder {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && (self.pbdi - self.ptdi).abs() > f32::EPSILON
    }
}

impl BrepShapeTrait for LCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        if !self.check_valid() {
            return None;
        }

        let dir = self.paxi_dir.normalize();
        let r = self.pdia / 2.0;
        let c_pt = dir * self.pbdi + self.paxi_pt;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let mut ext_len = self.ptdi - self.pbdi;
        let mut ext_dir = dir.vector3();
        if ext_len < 0.0 {
            ext_dir = -ext_dir;
            ext_len = -ext_len;
        }
        let v = builder::vertex(pt0.point3());
        let w = builder::rsweep(&v, center, ext_dir, Rad(7.0));
        let f = builder::try_attach_plane(&[w]).unwrap();
        let mut s = builder::tsweep(&f, ext_dir * ext_len as f64).into_boundaries();
        s.pop()
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLCylinder(self.clone()))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        // 确保返回正确的圆柱体哈希值，用于 unit_flag 判断
        CYLINDER_GEO_HASH
    }

    /// 如果是常规的基本体生成，直接跳过, 复用已经生成好的
    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        if !self.check_valid() {
            return Err(anyhow::anyhow!("Not valid LCylinder"));
        }

        Ok(CYLINDER_SHAPE.clone())
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Self::default())
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, (self.pbdi - self.ptdi).abs())
    }

    #[inline]
    fn get_trans(&self) -> Transform {
        let height = (self.ptdi - self.pbdi).abs();
        Transform {
            rotation: Default::default(),
            translation: if self.centre_line_flag {
                // 如果 centre_line_flag = true，paxi_pt 在圆柱体中心
                // 单位圆柱体的中心在 z=0.5，应用 scale 后中心在 z=height/2
                // 需要向下偏移 -height/2 使中心对齐到原点
                Vec3::new(0.0, 0.0, -height / 2.0)
            } else {
                // 如果 centre_line_flag = false，paxi_pt 在底部（pbdi位置）
                // 单位圆柱体的底部在 z=0，不需要偏移
                Vec3::ZERO
            },
            scale: self.get_scaled_vec3(),
        }
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        Some(crate::geometry::csg::unit_cylinder_mesh(
            &LodMeshSettings::default(),
            false,
        ))
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        let mut points = Vec::new();

        let dir = self.paxi_dir.normalize();
        let radius = self.pdia / 2.0;

        // 底面中心和顶面中心
        let bottom_center = self.paxi_pt + dir * self.pbdi;
        let top_center = self.paxi_pt + dir * self.ptdi;

        // 计算垂直于轴的两个向量
        let (u, v) = calculate_perpendicular_vectors(dir);

        // 1. 底面中心和顶面中心（优先级100）
        points.push((
            transform.transform_point(bottom_center),
            "Center".to_string(),
            100,
        ));
        points.push((
            transform.transform_point(top_center),
            "Center".to_string(),
            100,
        ));

        // 2. 底面和顶面的圆周点（8个点，优先级80）
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::PI / 4.0;
            let offset = u * angle.cos() * radius + v * angle.sin() * radius;

            // 底面圆周点
            points.push((
                transform.transform_point(bottom_center + offset),
                "Endpoint".to_string(),
                80,
            ));

            // 顶面圆周点
            points.push((
                transform.transform_point(top_center + offset),
                "Endpoint".to_string(),
                80,
            ));
        }

        // 3. 侧面中线点（4个点，优先级70）
        let mid_center = self.paxi_pt + dir * (self.pbdi + self.ptdi) / 2.0;
        for i in 0..4 {
            let angle = i as f32 * std::f32::consts::PI / 2.0;
            let offset = u * angle.cos() * radius + v * angle.sin() * radius;
            points.push((
                transform.transform_point(mid_center + offset),
                "Midpoint".to_string(),
                70,
            ));
        }

        points
    }
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
pub struct SCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    pub paxi_dir: Vec3,
    //dist to bottom
    pub phei: f32,
    // height
    pub pdia: f32,
    //diameter
    pub btm_shear_angles: [f32; 2],
    // x shear
    pub top_shear_angles: [f32; 2],
    // y shear
    pub negative: bool,
    pub center_in_mid: bool,
    /// 标识是否为单位化几何体（通过 transform 缩放而非 mesh 顶点缩放）
    pub unit_flag: bool,
    /// 当需要固定剪切方向的切向基时，可传入世界坐标系下的 (u, v) 基向量
    /// 用于避免 orthonormal_basis 选择不一致导致的 roll 跳变
    pub basis_hint: Option<[Vec3; 2]>,
}

impl Default for SCylinder {
    fn default() -> Self {
        Self {
            paxi_expr: "Z".to_string(),
            paxi_dir: Vec3::Z,
            paxi_pt: Default::default(),
            phei: 1.0,
            pdia: 1.0,
            btm_shear_angles: [0.0f32; 2],
            top_shear_angles: [0.0f32; 2],
            negative: false,
            center_in_mid: false,
            unit_flag: false,
            basis_hint: None,
        }
    }
}

impl SCylinder {
    #[inline]
    pub fn is_sscl(&self) -> bool {
        self.btm_shear_angles[0].abs() > f32::EPSILON
            || self.btm_shear_angles[1].abs() > f32::EPSILON
            || self.top_shear_angles[0].abs() > f32::EPSILON
            || self.top_shear_angles[1].abs() > f32::EPSILON
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}

impl BrepShapeTrait for SCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        let dir = self.paxi_dir.normalize();
        let dir = Vec3::Z;
        let r = self.pdia / 2.0;
        let c_pt = Vec3::ZERO;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let ext_len = self.phei as f64;
        let ext_dir = dir.vector3();
        let mut reverse_dir = false;
        if ext_len < 0.0 {
            reverse_dir = true;
        }
        // dbg!(ext_dir);
        let v = builder::vertex(pt0.point3());
        let origin_w = builder::rsweep(&v, center, ext_dir, Rad(7.0));

        //还是要和extrude 区分出来
        let scale_x = 1.0 / self.btm_shear_angles[0].to_radians().cos() as f64;
        let scale_y = 1.0 / self.btm_shear_angles[1].to_radians().cos() as f64;
        // dbg!(&self.btm_shear_angles);
        let transform_btm =
            Matrix4::from_angle_y(-Rad(self.btm_shear_angles[0].to_radians() as f64))
                * Matrix4::from_angle_x(Rad(self.btm_shear_angles[1].to_radians() as f64))
                * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);

        // dbg!(&self.top_shear_angles);
        let scale_x = 1.0 / self.top_shear_angles[0].to_radians().cos() as f64;
        let scale_y = 1.0 / self.top_shear_angles[1].to_radians().cos() as f64;
        let transform_top = Matrix4::from_translation(ext_dir * ext_len as f64)
            * Matrix4::from_angle_y(-Rad(self.top_shear_angles[0].to_radians() as f64))
            * Matrix4::from_angle_x(Rad(self.top_shear_angles[1].to_radians() as f64))
            * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);

        let mut w_s = builder::transformed(&origin_w, transform_btm);
        let mut w_e = builder::transformed(&origin_w, transform_top);
        if let Ok(mut f) = builder::try_attach_plane(&[w_s.clone()]) {
            let mut f_e = builder::try_attach_plane(&[w_e.clone()]).unwrap().inverse();
            // dbg!(reverse_dir);
            if !reverse_dir {
                f = f.inverse();
                f_e = f_e.inverse();
            }
            let h_w_s = w_s.split_off(w_s.len() / 2);
            let h_w_e = w_e.split_off(w_e.len() / 2);
            let face1 = builder::homotopy(w_s.front().unwrap(), &w_e.front().unwrap());
            let face2 = builder::homotopy(h_w_s.front().unwrap(), &h_w_e.front().unwrap());
            let shell = vec![f, f_e, face1, face2].into();
            return Some(shell);
        }
        None
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        if self.is_sscl() {
            vec![Vec3::ZERO.into(), (Vec3::Z * self.phei.abs()).into()]
        } else {
            vec![Vec3::ZERO.into(), (Vec3::Z * 1.0).into()]
        }
    }

    ///引用限制大小
    fn apply_limit_by_size(&mut self, l: f32) {
        self.phei = self.phei.min(l);
        // dbg!(self.phei);
        self.pdia = self.pdia.min(l);
    }

    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        if self.is_sscl() {
            // 对于斜切圆柱，使用 CSG 生成
            use crate::geometry::csg::generate_scylinder_mesh;
            use crate::mesh_precision::LodMeshSettings;
            if let Some(generated) =
                generate_scylinder_mesh(self, &LodMeshSettings::default(), false)
            {
                Ok(crate::prim_geo::basic::CsgSharedMesh::new(generated.mesh))
            } else {
                Err(anyhow::anyhow!("Failed to generate CSG mesh for SSCL"))
            }
        } else {
            Ok(CYLINDER_SHAPE.clone())
        }
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        if self.is_sscl() {
            let mut hasher = DefaultHasher::new();
            let bytes = bincode::serialize(self).unwrap();
            bytes.hash(&mut hasher);
            "SSCL".hash(&mut hasher);
            hasher.finish()
        } else {
            CYLINDER_GEO_HASH
        }
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        // 斜切圆柱不复用，但需要返回单位化版本
        let mut unit_shape = Self::default();
        unit_shape.unit_flag = true;
        Box::new(unit_shape)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_sscl() {
            Vec3::new(1.0, 1.0, 1.0)
        } else {
            Vec3::new(self.pdia, self.pdia, self.phei.abs())
        }
    }

    #[inline]
    fn get_trans(&self) -> Transform {
        Transform {
            rotation: Default::default(),
            translation: if self.center_in_mid {
                Vec3::new(0.0, 0.0, -self.phei / 2.0)
            } else {
                Vec3::ZERO
            },
            scale: self.get_scaled_vec3(),
        }
    }

    #[inline]
    fn tol(&self) -> f32 {
        if self.is_sscl() {
            0.004 * (self.pdia.max(1.0))
        } else {
            TRI_TOL
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimSCylinder(self.clone()))
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        Some(crate::geometry::csg::unit_cylinder_mesh(
            &LodMeshSettings::default(),
            false,
        ))
    }

    /// 为圆柱体生成增强的关键点
    ///
    /// 包括：
    /// - 2个面中心（顶面和底面，优先级100）
    /// - 16个圆周点（顶面8个+底面8个，优先级80）
    /// - 8个侧面中线点（优先级70）
    ///
    /// 对于 SSCL（倾斜圆柱体），会根据 btm_shear_angles/top_shear_angles 计算真实的
    /// 顶/底面中心与法向，确保关键点与实际几何一致。
    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        let mut points = Vec::new();

        // 归一化轴向向量，处理零向量情况
        let dir = self.paxi_dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            // 如果轴向无效，返回空列表
            return points;
        }

        let radius = self.pdia / 2.0;
        let height = self.phei.abs(); // 使用绝对值处理符号
        let half_height = height / 2.0;

        // 根据 center_in_mid 确定基准点
        let base_center = if self.center_in_mid {
            self.paxi_pt // 中心在中间
        } else {
            self.paxi_pt + dir * half_height // 中心在底面，需要偏移到中点
        };

        // 计算垂直于轴的两个正交向量
        let (u, v) = calculate_perpendicular_vectors(dir);

        // 检查是否为 SSCL（倾斜圆柱体）
        if self.is_sscl() {
            // SSCL 处理：根据剪切角度计算真实的顶/底面位置
            let btm_shear_x = self.btm_shear_angles[0].to_radians();
            let btm_shear_y = self.btm_shear_angles[1].to_radians();
            let top_shear_x = self.top_shear_angles[0].to_radians();
            let top_shear_y = self.top_shear_angles[1].to_radians();

            // 底面中心偏移（由剪切角度引起）
            let btm_offset = u * (radius * btm_shear_x.tan()) + v * (radius * btm_shear_y.tan());
            let bottom_center = base_center - dir * half_height + btm_offset;

            // 顶面中心偏移
            let top_offset = u * (radius * top_shear_x.tan()) + v * (radius * top_shear_y.tan());
            let top_center = base_center + dir * half_height + top_offset;

            // 底面中心（优先级：100）
            points.push((
                transform.transform_point(bottom_center),
                "Center".to_string(),
                100,
            ));

            // 顶面中心（优先级：100）
            points.push((
                transform.transform_point(top_center),
                "Center".to_string(),
                100,
            ));

            // 底面圆周8个点（考虑剪切，优先级：80）
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let base_offset = (u * angle.cos() + v * angle.sin()) * radius;
                // 剪切面上的点需要沿轴向调整
                let shear_z = base_offset.dot(u) * btm_shear_x.tan()
                    + base_offset.dot(v) * btm_shear_y.tan();
                let local_pos = bottom_center + base_offset + dir * shear_z;
                points.push((
                    transform.transform_point(local_pos),
                    "Endpoint".to_string(),
                    80,
                ));
            }

            // 顶面圆周8个点（考虑剪切，优先级：80）
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let base_offset = (u * angle.cos() + v * angle.sin()) * radius;
                let shear_z = base_offset.dot(u) * top_shear_x.tan()
                    + base_offset.dot(v) * top_shear_y.tan();
                let local_pos = top_center + base_offset + dir * shear_z;
                points.push((
                    transform.transform_point(local_pos),
                    "Endpoint".to_string(),
                    80,
                ));
            }

            // 侧面中线8个点（优先级：70）
            let mid_center = (bottom_center + top_center) / 2.0;
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let offset = (u * angle.cos() + v * angle.sin()) * radius;
                points.push((
                    transform.transform_point(mid_center + offset),
                    "Midpoint".to_string(),
                    70,
                ));
            }
        } else {
            // 普通圆柱体处理
            let bottom_center = base_center - dir * half_height;
            let top_center = base_center + dir * half_height;

            // 底面中心（优先级：100）
            points.push((
                transform.transform_point(bottom_center),
                "Center".to_string(),
                100,
            ));

            // 顶面中心（优先级：100）
            points.push((
                transform.transform_point(top_center),
                "Center".to_string(),
                100,
            ));

            // 顶面圆周8个点（优先级：80）
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let offset = (u * angle.cos() + v * angle.sin()) * radius;
                let local_pos = top_center + offset;
                points.push((
                    transform.transform_point(local_pos),
                    "Endpoint".to_string(),
                    80,
                ));
            }

            // 底面圆周8个点（优先级：80）
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let offset = (u * angle.cos() + v * angle.sin()) * radius;
                let local_pos = bottom_center + offset;
                points.push((
                    transform.transform_point(local_pos),
                    "Endpoint".to_string(),
                    80,
                ));
            }

            // 侧面中线8个点（优先级：70）
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let offset = (u * angle.cos() + v * angle.sin()) * radius;
                let local_pos = base_center + offset;
                points.push((
                    transform.transform_point(local_pos),
                    "Midpoint".to_string(),
                    70,
                ));
            }
        }

        points
    }
}

/// 计算垂直于给定轴的两个正交向量
///
/// 返回：(u, v) 两个单位向量，满足 u ⊥ axis, v ⊥ axis, u ⊥ v
fn calculate_perpendicular_vectors(axis: Vec3) -> (Vec3, Vec3) {
    let axis = axis.normalize();
    // 选择一个不平行于轴的向量
    let reference = if axis.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = reference.cross(axis).normalize();
    let v = axis.cross(u).normalize();
    (u, v)
}

impl From<&AttrMap> for SCylinder {
    fn from(m: &AttrMap) -> Self {
        let phei = m.get_f32_or_default("HEIG");
        let pdia = m.get_f32_or_default("DIAM");
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            negative: false,
            center_in_mid: true,
            unit_flag: false,
            ..Default::default()
        }
    }
}

impl From<AttrMap> for SCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for SCylinder {
    fn from(m: &NamedAttrMap) -> Self {
        let phei = m.get_f32_or_default("HEIG");
        let pdia = m.get_f32_or_default("DIAM");
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            negative: false,
            center_in_mid: true,
            unit_flag: false,
            ..Default::default()
        }
    }
}

impl From<NamedAttrMap> for SCylinder {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
