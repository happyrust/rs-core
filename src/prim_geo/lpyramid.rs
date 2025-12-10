use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;

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
pub struct LPyramid {
    pub pbax_pt: Vec3,
    pub pbax_dir: Vec3, //B Axis Direction

    pub pcax_pt: Vec3,
    pub pcax_dir: Vec3, //C Axis Direction

    pub paax_pt: Vec3,
    pub paax_dir: Vec3, //A Axis Direction

    pub pbtp: f32,
    pub pctp: f32,
    //y top
    pub pbbt: f32,
    pub pcbt: f32, // y bottom

    pub ptdi: f32,
    pub pbdi: f32,
    pub pbof: f32,
    // x offset
    pub pcof: f32, // y offset
}

impl Default for LPyramid {
    fn default() -> Self {
        Self {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: 1.0,
            pctp: 1.0,
            pbbt: 1.0,
            pcbt: 1.0,
            ptdi: 1.0,
            pbdi: 0.0,
            pbof: 0.0,
            pcof: 0.0,
        }
    }
}

impl VerifiedShape for LPyramid {
    fn check_valid(&self) -> bool {
        let size_flag =
            self.pbtp * self.pctp >= f32::EPSILON || self.pbbt * self.pcbt >= f32::EPSILON;
        if !size_flag {
            return false;
        }
        (self.pbtp >= 0.0 && self.pctp >= 0.0 && self.pbbt >= 0.0 && self.pcbt >= 0.0)
            && ((self.pbtp + self.pctp) > f32::EPSILON || (self.pbbt + self.pcbt) > f32::EPSILON)
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LPyramid {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "LPyramid".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLPyramid(self.clone()))
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        use crate::geometry::csg::{orthonormal_basis, safe_normalize};

        let mut points = Vec::new();

        // 正交化轴方向（与 mesh 生成保持一致）
        let axis_dir = match safe_normalize(self.paax_dir) {
            Some(d) => d,
            None => return points,
        };
        let (fallback_u, fallback_v) = orthonormal_basis(axis_dir);

        // 正交化 B 轴方向
        let mut pb_dir = safe_normalize(self.pbax_dir).unwrap_or(fallback_u);
        pb_dir = (pb_dir - axis_dir * pb_dir.dot(axis_dir)).normalize_or_zero();
        if pb_dir.length_squared() <= 0.0001 {
            pb_dir = fallback_u;
        }

        // 正交化 C 轴方向
        let mut pc_dir = safe_normalize(self.pcax_dir).unwrap_or(fallback_v);
        pc_dir = (pc_dir - axis_dir * pc_dir.dot(axis_dir) - pb_dir * pc_dir.dot(pb_dir))
            .normalize_or_zero();
        if pc_dir.length_squared() <= 0.0001 {
            pc_dir = fallback_v;
        }

        // 偏移使用正交化后的方向（与 mesh 生成一致）
        let offset_3d = pb_dir * self.pbof + pc_dir * self.pcof;

        // 底面中心（参考点）
        let bottom_center = self.paax_pt + axis_dir * self.pbdi;
        // 顶面中心（带偏移）
        let height = self.ptdi - self.pbdi;
        let top_center = bottom_center + axis_dir * height + offset_3d;

        // 1. 顶面和底面中心（优先级100）
        points.push((
            transform.transform_point(top_center),
            "Center".to_string(),
            100,
        ));
        points.push((
            transform.transform_point(bottom_center),
            "Center".to_string(),
            100,
        ));

        // 2. 顶面的4个顶点（如果不是退化为点）
        let tx = self.pbtp / 2.0;
        let ty = self.pctp / 2.0;
        if tx > 0.001 && ty > 0.001 {
            let top_corners = [
                top_center + pb_dir * tx + pc_dir * ty,
                top_center + pb_dir * tx - pc_dir * ty,
                top_center - pb_dir * tx + pc_dir * ty,
                top_center - pb_dir * tx - pc_dir * ty,
            ];
            for corner in &top_corners {
                points.push((
                    transform.transform_point(*corner),
                    "Endpoint".to_string(),
                    90,
                ));
            }
        }

        // 3. 底面的4个顶点
        let bx = self.pbbt / 2.0;
        let by = self.pcbt / 2.0;
        if bx > 0.001 && by > 0.001 {
            let bottom_corners = [
                bottom_center + pb_dir * bx + pc_dir * by,
                bottom_center + pb_dir * bx - pc_dir * by,
                bottom_center - pb_dir * bx + pc_dir * by,
                bottom_center - pb_dir * bx - pc_dir * by,
            ];
            for corner in &bottom_corners {
                points.push((
                    transform.transform_point(*corner),
                    "Endpoint".to_string(),
                    90,
                ));
            }
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    /// 创建测试用的 LPyramid 辅助函数
    fn create_lpyramid(
        pbtp: f32, pctp: f32, pbbt: f32, pcbt: f32,
        ptdi: f32, pbdi: f32, pbof: f32, pcof: f32
    ) -> LPyramid {
        LPyramid {
            pbax_pt: Vec3::ZERO,
            pbax_dir: Vec3::X,
            pcax_pt: Vec3::ZERO,
            pcax_dir: Vec3::Y,
            paax_pt: Vec3::ZERO,
            paax_dir: Vec3::Z,
            pbtp,
            pctp,
            pbbt,
            pcbt,
            ptdi,
            pbdi,
            pbof,
            pcof,
        }
    }

    #[test]
    fn test_lpyramid_standard_pyramid() {
        // 场景1: 标准矩形棱锥
        // XTOP = 0, YTOP = 0, OFFX = 0, OFFY = 0
        let pyramid = create_lpyramid(
            0.0, 0.0,  // 顶面尺寸为0（尖顶）
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0   // 无偏移
        );

        assert!(pyramid.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);
        
        // 应该有2个中心点（顶面和底面）
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 顶面退化为点，不应有顶面顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 4); // 只有底面4个顶点
    }

    #[test]
    fn test_lpyramid_frustum() {
        // 场景2: 矩形棱台
        // XTOP > 0, YTOP > 0, OFFX = 0, OFFY = 0
        let frustum = create_lpyramid(
            2.0, 3.0,  // 顶面尺寸
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0   // 无偏移
        );

        assert!(frustum.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = frustum.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 应该有8个顶点（顶面4个 + 底面4个）
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn test_lpyramid_wedge_x() {
        // 场景3: 楔形体（X方向脊线）
        // XTOP > 0, YTOP = 0
        let wedge = create_lpyramid(
            2.0, 0.0,  // 顶面Y尺寸为0
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0   // 无偏移
        );

        assert!(wedge.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = wedge.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 顶面退化为线，只有底面4个顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 4);
    }

    #[test]
    fn test_lpyramid_wedge_y() {
        // 场景3: 楔形体（Y方向脊线）
        // XTOP = 0, YTOP > 0
        let wedge = create_lpyramid(
            0.0, 3.0,  // 顶面X尺寸为0
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0   // 无偏移
        );

        assert!(wedge.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = wedge.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 顶面退化为线，只有底面4个顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 4);
    }

    #[test]
    fn test_lpyramid_oblique_pyramid() {
        // 场景4: 斜棱锥
        // OFFX != 0 或 OFFY != 0
        let oblique = create_lpyramid(
            0.0, 0.0,  // 顶面尺寸为0（尖顶）
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            1.5, 2.0   // 有偏移
        );

        assert!(oblique.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = oblique.enhanced_key_points(&transform);
        
        // 应该有2个中心点（顶面中心因偏移而移动）
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 检查顶面中心是否偏移
        let top_center = centers[0].0;
        assert!(top_center.x.abs() > 0.1); // X方向有偏移
        assert!(top_center.y.abs() > 0.1); // Y方向有偏移
        assert!(top_center.z > 9.0);        // Z方向高度正确
        
        // 顶面退化为点，只有底面4个顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 4);
    }

    #[test]
    fn test_lpyramid_oblique_frustum() {
        // 场景4: 斜棱台
        // XTOP > 0, YTOP > 0, OFFX != 0 或 OFFY != 0
        let oblique = create_lpyramid(
            2.0, 3.0,  // 顶面尺寸
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            1.0, -1.5  // 有偏移
        );

        assert!(oblique.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = oblique.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 检查顶面中心是否偏移
        let top_center = centers[0].0;
        assert!(top_center.x.abs() > 0.1); // X方向有偏移
        assert!(top_center.y.abs() > 0.1); // Y方向有偏移
        
        // 应该有8个顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn test_lpyramid_general_prism() {
        // 场景5: 一般性非均匀棱台
        // 顶底面长宽比不同
        let prism = create_lpyramid(
            1.0, 4.0,  // 顶面尺寸（长宽比与底面不同）
            4.0, 6.0,  // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0   // 无偏移
        );

        assert!(prism.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = prism.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 应该有8个顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn test_lpyramid_invalid_zero_height() {
        // 边界情况：高度为0
        let pyramid = create_lpyramid(
            2.0, 3.0,
            4.0, 6.0,
            0.0, 0.0,  // 高度为0
            0.0, 0.0
        );

        // 高度为0但其他尺寸有效，应该仍然有效（退化为平面）
        assert!(pyramid.check_valid());
    }

    #[test]
    fn test_lpyramid_inverted_pyramid() {
        // 边界情况：底面尺寸为0（倒立棱锥）
        // 根据 check_valid 逻辑：顶面或底面至少有一个有面积即可
        let pyramid = create_lpyramid(
            2.0, 3.0,  // 顶面有效
            0.0, 0.0,  // 底面尺寸为0
            10.0, 0.0,
            0.0, 0.0
        );

        // 顶面有效，底面为0 = 倒立棱锥，应该有效
        assert!(pyramid.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 顶面有4个顶点，底面退化为点无顶点
        let endpoints: Vec<_> = points.iter().filter(|(_, name, _)| name == "Endpoint").collect();
        assert_eq!(endpoints.len(), 4);
        
        println!("✅ 倒立棱锥 (底面退化为点) 验证通过");
    }

    #[test]
    fn test_lpyramid_invalid_both_zero() {
        // 边界情况：顶面和底面都为0
        let pyramid = create_lpyramid(
            0.0, 0.0,  // 顶面为0
            0.0, 0.0,  // 底面为0
            10.0, 0.0,
            0.0, 0.0
        );

        // 应该无效
        assert!(!pyramid.check_valid());
    }

    #[test]
    fn test_lpyramid_negative_dimensions() {
        // 边界情况：负尺寸
        let pyramid = create_lpyramid(
            -2.0, 3.0,  // 负的顶面尺寸
            4.0, 6.0,
            10.0, 0.0,
            0.0, 0.0
        );

        // 负尺寸应该无效
        assert!(!pyramid.check_valid());
    }

    #[test]
    fn test_lpyramid_large_offset() {
        // 边界情况：极大偏移
        let pyramid = create_lpyramid(
            2.0, 3.0,
            4.0, 6.0,
            10.0, 0.0,
            100.0, 100.0  // 极大偏移
        );

        // 大偏移应该仍然有效
        assert!(pyramid.check_valid());
        
        // 验证关键点位置
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);
        
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        let top_center = centers[0].0;
        
        // 偏移应该正确反映
        assert!(top_center.x.abs() > 50.0);
        assert!(top_center.y.abs() > 50.0);
    }

    #[test]
    fn test_lpyramid_complex_scenario() {
        // 复杂组合场景：非均匀棱台 + 斜偏移
        let complex = create_lpyramid(
            1.5, 2.5,  // 顶面尺寸
            5.0, 8.0,  // 底面尺寸（比例不同）
            15.0, 2.0, // 高度，底部不在0
            2.5, -3.0  // 复杂偏移
        );

        assert!(complex.check_valid());
        
        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = complex.enhanced_key_points(&transform);
        
        // 应该有2个中心点
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
        
        // 验证底部中心位置（考虑pbdi）
        let bottom_center = centers[1].0;
        assert!(bottom_center.z > 1.5); // 底部在z=2.0附近
        
        // 验证顶部中心位置（考虑高度和偏移）
        let top_center = centers[0].0;
        assert!(top_center.z > 15.0); // 顶部在z=17.0附近
        assert!(top_center.x > 2.0);  // X偏移
        assert!(top_center.y < -2.0); // Y负偏移
    }

    #[test]
    fn test_lpyramid_hash_consistency() {
        // 测试哈希一致性
        let pyramid1 = create_lpyramid(
            2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 1.0, 2.0
        );
        let pyramid2 = create_lpyramid(
            2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 1.0, 2.0
        );
        let pyramid3 = create_lpyramid(
            2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 1.1, 2.0
        );

        // 相同参数应该有相同哈希
        assert_eq!(pyramid1.hash_unit_mesh_params(), pyramid2.hash_unit_mesh_params());
        
        // 不同参数应该有不同哈希
        assert_ne!(pyramid1.hash_unit_mesh_params(), pyramid3.hash_unit_mesh_params());
    }

    #[test]
    fn test_lpyramid_axis_orientation() {
        // 测试轴方向对关键点的影响
        let mut pyramid = create_lpyramid(
            0.0, 0.0, 4.0, 6.0, 10.0, 0.0, 0.0, 0.0
        );
        
        // 改变轴方向
        pyramid.paax_dir = Vec3::new(0.0, 1.0, 0.0); // A轴指向Y
        pyramid.pbax_dir = Vec3::new(1.0, 0.0, 0.0); // B轴指向X
        pyramid.pcax_dir = Vec3::new(0.0, 0.0, 1.0); // C轴指向Z
        
        assert!(pyramid.check_valid());
        
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);
        
        // 应该仍然有正确的点数
        let centers: Vec<_> = points.iter().filter(|(_, name, _)| name == "Center").collect();
        assert_eq!(centers.len(), 2);
    }
}
