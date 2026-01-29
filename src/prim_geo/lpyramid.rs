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
        // 关键点（key points）之算式，须与 CSG `generate_lpyramid_mesh` 同步：
        // - 彼处生成网格之顶点集合，乃后续布尔/碰撞/选取等之“几何真相”。
        // - 此处若用另一套轴系/偏移/中心定义，则关键点与网格不合，易致拾取点漂移。
        //
        // CSG 约定（见 `geometry::csg::generate_lpyramid_mesh`）：
        // - 局部坐标系固定：A轴(高度) = +Z，B轴(宽) = +X，C轴(深) = +Y
        // - 底面中心固定为原点：bottom_center = (0,0,0)
        // - 高度仅取差值：height = PTDI - PBDI（注意：PBDI 本身不直接体现在局部 Z）
        // - 顶面偏移仅作用于顶面：offset_3d = (PBOF * X) + (PCOF * Y)
        // - 顶点顺序固定：(-1,-1),(+1,-1),(+1,+1),(-1,+1)
        //
        // 此函数只负责“点集”，不负责三角形/法线；最终通过 `transform` 映射至世界坐标。
        const MIN_LEN: f32 = 0.001;
        let mut points = Vec::new();

        // 局部轴：与 CSG 完全一致（此处不使用 paax_dir/pbax_dir/pcax_dir 做正交化）。
        let axis_dir = Vec3::Z;
        let pb_dir = Vec3::X;
        let pc_dir = Vec3::Y;

        // 半尺寸：与 CSG 中 tx/ty/bx/by 语义一致。
        let tx = self.pbtp * 0.5;
        let ty = self.pctp * 0.5;
        let bx = self.pbbt * 0.5;
        let by = self.pcbt * 0.5;

        // 顶面偏移：仅顶面带 offset，底面无 offset。
        let offset_3d = pb_dir * self.pbof + pc_dir * self.pcof;

        // CSG 以底面中心为原点（center 恒为 ZERO）。
        // 若未来 CSG 改为使用 `paax_pt` 或 `pbdi` 参与局部原点，此处亦须同改。
        let center = Vec3::ZERO;
        let height = self.ptdi - self.pbdi;

        // 顶点顺序：与 CSG `offsets` 数组一致；保持顺序可使调试对比更直观。
        let offsets: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

        // 顶/底中心：
        // - bottom_center = (0,0,0)
        // - top_center.z = height，且额外叠加 offset_3d
        let top_center = center + axis_dir * height + offset_3d;
        let bottom_center = center;

        // 中心点（优先级 100）：用于整体定位与快速选择。
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

        // 顶面四角：当顶面退化（tx 或 ty 太小）时，CSG 会走 apex 分支；
        // 关键点此处亦同步：顶面退化则不输出顶面四角。
        if tx > MIN_LEN && ty > MIN_LEN {
            for (ox, oy) in offsets.iter() {
                let pos = center
                    + pb_dir * (ox * tx)
                    + pc_dir * (oy * ty)
                    + axis_dir * height
                    + offset_3d;
                points.push((transform.transform_point(pos), "Endpoint".to_string(), 90));
            }
        }

        // 底面四角：底面不带 offset；底面退化则不输出底面四角。
        if bx > MIN_LEN && by > MIN_LEN {
            for (ox, oy) in offsets.iter() {
                let pos = center + pb_dir * (ox * bx) + pc_dir * (oy * by);
                points.push((transform.transform_point(pos), "Endpoint".to_string(), 90));
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
        pbtp: f32,
        pctp: f32,
        pbbt: f32,
        pcbt: f32,
        ptdi: f32,
        pbdi: f32,
        pbof: f32,
        pcof: f32,
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
            0.0, 0.0, // 顶面尺寸为0（尖顶）
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0, // 无偏移
        );

        assert!(pyramid.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);

        // 应该有2个中心点（顶面和底面）
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 顶面退化为点，不应有顶面顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 4); // 只有底面4个顶点
    }

    #[test]
    fn test_lpyramid_frustum() {
        // 场景2: 矩形棱台
        // XTOP > 0, YTOP > 0, OFFX = 0, OFFY = 0
        let frustum = create_lpyramid(
            2.0, 3.0, // 顶面尺寸
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0, // 无偏移
        );

        assert!(frustum.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = frustum.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 应该有8个顶点（顶面4个 + 底面4个）
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn test_lpyramid_wedge_x() {
        // 场景3: 楔形体（X方向脊线）
        // XTOP > 0, YTOP = 0
        let wedge = create_lpyramid(
            2.0, 0.0, // 顶面Y尺寸为0
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0, // 无偏移
        );

        assert!(wedge.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = wedge.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 顶面退化为线，只有底面4个顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 4);
    }

    #[test]
    fn test_lpyramid_wedge_y() {
        // 场景3: 楔形体（Y方向脊线）
        // XTOP = 0, YTOP > 0
        let wedge = create_lpyramid(
            0.0, 3.0, // 顶面X尺寸为0
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0, // 无偏移
        );

        assert!(wedge.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = wedge.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 顶面退化为线，只有底面4个顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 4);
    }

    #[test]
    fn test_lpyramid_oblique_pyramid() {
        // 场景4: 斜棱锥
        // OFFX != 0 或 OFFY != 0
        let oblique = create_lpyramid(
            0.0, 0.0, // 顶面尺寸为0（尖顶）
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            1.5, 2.0, // 有偏移
        );

        assert!(oblique.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = oblique.enhanced_key_points(&transform);

        // 应该有2个中心点（顶面中心因偏移而移动）
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 检查顶面中心是否偏移
        let top_center = centers[0].0;
        assert!(top_center.x.abs() > 0.1); // X方向有偏移
        assert!(top_center.y.abs() > 0.1); // Y方向有偏移
        assert!(top_center.z > 9.0); // Z方向高度正确

        // 顶面退化为点，只有底面4个顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 4);
    }

    #[test]
    fn test_lpyramid_oblique_frustum() {
        // 场景4: 斜棱台
        // XTOP > 0, YTOP > 0, OFFX != 0 或 OFFY != 0
        let oblique = create_lpyramid(
            2.0, 3.0, // 顶面尺寸
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            1.0, -1.5, // 有偏移
        );

        assert!(oblique.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = oblique.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 检查顶面中心是否偏移
        let top_center = centers[0].0;
        assert!(top_center.x.abs() > 0.1); // X方向有偏移
        assert!(top_center.y.abs() > 0.1); // Y方向有偏移

        // 应该有8个顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn test_lpyramid_general_prism() {
        // 场景5: 一般性非均匀棱台
        // 顶底面长宽比不同
        let prism = create_lpyramid(
            1.0, 4.0, // 顶面尺寸（长宽比与底面不同）
            4.0, 6.0, // 底面尺寸
            10.0, 0.0, // 高度
            0.0, 0.0, // 无偏移
        );

        assert!(prism.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = prism.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 应该有8个顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn test_lpyramid_invalid_zero_height() {
        // 边界情况：高度为0
        let pyramid = create_lpyramid(
            2.0, 3.0, 4.0, 6.0, 0.0, 0.0, // 高度为0
            0.0, 0.0,
        );

        // 高度为0但其他尺寸有效，应该仍然有效（退化为平面）
        assert!(pyramid.check_valid());
    }

    #[test]
    fn test_lpyramid_inverted_pyramid() {
        // 边界情况：底面尺寸为0（倒立棱锥）
        // 根据 check_valid 逻辑：顶面或底面至少有一个有面积即可
        let pyramid = create_lpyramid(
            2.0, 3.0, // 顶面有效
            0.0, 0.0, // 底面尺寸为0
            10.0, 0.0, 0.0, 0.0,
        );

        // 顶面有效，底面为0 = 倒立棱锥，应该有效
        assert!(pyramid.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 顶面有4个顶点，底面退化为点无顶点
        let endpoints: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Endpoint")
            .collect();
        assert_eq!(endpoints.len(), 4);

        println!("✅ 倒立棱锥 (底面退化为点) 验证通过");
    }

    #[test]
    fn test_lpyramid_invalid_both_zero() {
        // 边界情况：顶面和底面都为0
        let pyramid = create_lpyramid(
            0.0, 0.0, // 顶面为0
            0.0, 0.0, // 底面为0
            10.0, 0.0, 0.0, 0.0,
        );

        // 应该无效
        assert!(!pyramid.check_valid());
    }

    #[test]
    fn test_lpyramid_negative_dimensions() {
        // 边界情况：负尺寸
        let pyramid = create_lpyramid(
            -2.0, 3.0, // 负的顶面尺寸
            4.0, 6.0, 10.0, 0.0, 0.0, 0.0,
        );

        // 负尺寸应该无效
        assert!(!pyramid.check_valid());
    }

    #[test]
    fn test_lpyramid_large_offset() {
        // 边界情况：极大偏移
        let pyramid = create_lpyramid(
            2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 100.0, 100.0, // 极大偏移
        );

        // 大偏移应该仍然有效
        assert!(pyramid.check_valid());

        // 验证关键点位置
        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);

        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        let top_center = centers[0].0;

        // 偏移应该正确反映
        assert!(top_center.x.abs() > 50.0);
        assert!(top_center.y.abs() > 50.0);
    }

    #[test]
    fn test_lpyramid_complex_scenario() {
        // 复杂组合场景：非均匀棱台 + 斜偏移
        let complex = create_lpyramid(
            1.5, 2.5, // 顶面尺寸
            5.0, 8.0, // 底面尺寸（比例不同）
            15.0, 2.0, // 高度，底部不在0
            2.5, -3.0, // 复杂偏移
        );

        assert!(complex.check_valid());

        // 验证关键点
        let transform = bevy_transform::prelude::Transform::default();
        let points = complex.enhanced_key_points(&transform);

        // 应该有2个中心点
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);

        // 验证顶部中心位置（CSG 坐标：bottom_center = 0, top_center.z = ptdi - pbdi）
        let top_center = centers[0].0;
        assert!(top_center.z > 12.0); // height = 15.0 - 2.0 = 13.0
        assert!(top_center.x > 2.0); // X偏移
        assert!(top_center.y < -2.0); // Y负偏移
    }

    #[test]
    fn test_lpyramid_hash_consistency() {
        // 测试哈希一致性
        let pyramid1 = create_lpyramid(2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 1.0, 2.0);
        let pyramid2 = create_lpyramid(2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 1.0, 2.0);
        let pyramid3 = create_lpyramid(2.0, 3.0, 4.0, 6.0, 10.0, 0.0, 1.1, 2.0);

        // 相同参数应该有相同哈希
        assert_eq!(
            pyramid1.hash_unit_mesh_params(),
            pyramid2.hash_unit_mesh_params()
        );

        // 不同参数应该有不同哈希
        assert_ne!(
            pyramid1.hash_unit_mesh_params(),
            pyramid3.hash_unit_mesh_params()
        );
    }

    #[test]
    fn test_lpyramid_axis_orientation() {
        // 测试轴方向对关键点的影响
        let mut pyramid = create_lpyramid(0.0, 0.0, 4.0, 6.0, 10.0, 0.0, 0.0, 0.0);

        // 改变轴方向
        pyramid.paax_dir = Vec3::new(0.0, 1.0, 0.0); // A轴指向Y
        pyramid.pbax_dir = Vec3::new(1.0, 0.0, 0.0); // B轴指向X
        pyramid.pcax_dir = Vec3::new(0.0, 0.0, 1.0); // C轴指向Z

        assert!(pyramid.check_valid());

        let transform = bevy_transform::prelude::Transform::default();
        let points = pyramid.enhanced_key_points(&transform);

        // 应该仍然有正确的点数
        let centers: Vec<_> = points
            .iter()
            .filter(|(_, name, _)| name == "Center")
            .collect();
        assert_eq!(centers.len(), 2);
    }
}
