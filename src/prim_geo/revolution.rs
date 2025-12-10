use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::wire::*;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32, hash_vec3};
use approx::AbsDiffEq;
use approx::abs_diff_eq;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
#[cfg(feature = "truck")]
use truck_modeling::{Surface, builder};
#[cfg(feature = "truck")]
use truck_stepio::out;

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct Revolution {
    /// 轮廓顶点，PDMS 格式：x=轴向(高度), y=径向, z=FRAD
    pub verts: Vec<Vec<Vec3>>,
    /// 旋转角度（度）
    pub angle: f32,
}

impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![vec![Vec3::new(0.0, 0.0, 0.0)]],
            angle: 360.0,
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool {
        self.angle.abs() > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }


    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().flatten().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(self.angle, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        use parry2d::bounding_volume::Aabb;
        let pts = self
            .verts
            .iter()
            .flatten()
            .map(|x| nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate())))
            .collect::<Vec<_>>();
        let profile_aabb = Aabb::from_points(pts.iter().copied());
        0.001 * profile_aabb.bounding_sphere().radius.max(1.0)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimRevolution(self.clone()))
    }

    /// 使用 Manifold 风格算法生成旋转体的 mesh
    ///
    /// 特性：
    /// - 默认绕 X 轴旋转（PDMS 数据格式：x=高度/轴向，y=径向，z=FRAD）
    /// - 自动处理 FRAD 圆角（verts.z）
    /// - 自动裁剪负径向侧轮廓
    /// - 轴上顶点优化（径向=0 的点不重复复制）
    /// - 自适应分段数
    /// - 支持部分旋转（非 360°）的端面封闭
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        if !self.check_valid() {
            return None;
        }
        if self.verts.is_empty() || self.verts[0].len() < 3 {
            return None;
        }

        use crate::prim_geo::profile_processor::{ProfileProcessor, revolve_polygons_manifold};

        // 检查是否有 FRAD 需要处理（verts.z != 0）
        let has_frad = self.verts.iter().flatten().any(|v| v.z.abs() > 0.01);

        let polygons: Vec<Vec<Vec2>> = if has_frad {
            // 使用 ProfileProcessor 处理 FRAD 圆角
            // PDMS 格式：verts.x = 轴向, verts.y = 径向, verts.z = FRAD
            self.verts
                .iter()
                .filter_map(|wire| {
                    let processor = ProfileProcessor::new_single(wire.clone());
                    match processor.process("revolution", None) {
                        Ok(processed) => {
                            // 处理后的点：x=轴向, y=径向（已展开圆角）
                            // 转换为 libgm 2D profile：profile.x=径向, profile.y=轴向
                            Some(
                                processed
                                    .contour_points
                                    .iter()
                                    .map(|p| Vec2::new(p.y, p.x))
                                    .collect(),
                            )
                        }
                        Err(_) => {
                            // ProfileProcessor 失败，回退到直接转换
                            Some(wire.iter().map(|p| Vec2::new(p.y, p.x)).collect())
                        }
                    }
                })
                .collect()
        } else {
            // 无 FRAD，直接转换
            // PDMS 格式：verts.x = 轴向, verts.y = 径向
            // libgm 2D profile：profile.x = 径向, profile.y = 轴向
            self.verts
                .iter()
                .map(|wire| wire.iter().map(|p| Vec2::new(p.y, p.x)).collect())
                .collect()
        };

        // 使用 Manifold 风格的旋转生成算法
        // segments = 0 表示使用自适应分段数
        let revolved = revolve_polygons_manifold(&polygons, 0, self.angle)?;

        Some(PlantMesh {
            vertices: revolved.vertices,
            normals: revolved.normals,
            uvs: revolved.uvs,
            indices: revolved.indices,
            wire_vertices: Vec::new(),
            edges: Vec::new(),
            aabb: None,
        })
    }

    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        if let Some(mesh) = self.gen_csg_mesh() {
            Ok(crate::prim_geo::basic::CsgSharedMesh::new(mesh))
        } else {
            Err(anyhow::anyhow!(
                "Failed to generate CSG mesh for Revolution"
            ))
        }
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        use glam::Quat;

        let mut points = Vec::new();

        // 1. 旋转中心点（优先级100）- 固定在原点
        points.push((
            transform.transform_point(Vec3::ZERO),
            "Center".to_string(),
            100,
        ));

        // 获取所有 profile 顶点
        let all_verts: Vec<Vec3> = self.verts.iter().flatten().cloned().collect();
        if all_verts.is_empty() {
            return points;
        }

        // libgm 内部绕 Y 轴旋转
        // PDMS 格式：verts.x = 轴向, verts.y = 径向
        // 3D 起始点 (θ=0)：(径向, 轴向, 0) = (verts.y, verts.x, 0)
        let rot_axis = Vec3::Y;
        let angle_rad = self.angle.to_radians();

        // 2. 起始面 profile 顶点（优先级90）
        for v in &all_verts {
            let start_pt = Vec3::new(v.y, v.x, 0.0);
            points.push((
                transform.transform_point(start_pt),
                "Endpoint".to_string(),
                90,
            ));
        }

        // 3. 终止面 profile 顶点（旋转后，优先级90）
        let end_rotation = Quat::from_axis_angle(rot_axis, angle_rad);
        for v in &all_verts {
            let start_pt = Vec3::new(v.y, v.x, 0.0);
            let rotated_pt = end_rotation * start_pt;
            points.push((
                transform.transform_point(rotated_pt),
                "Endpoint".to_string(),
                90,
            ));
        }

        // 4. 中间角度的采样点（优先级70）- 在 1/4, 1/2, 3/4 位置
        for fraction in [0.25, 0.5, 0.75] {
            let mid_angle = angle_rad * fraction;
            let mid_rotation = Quat::from_axis_angle(rot_axis, mid_angle);

            for v in all_verts.iter().take(4) {
                let start_pt = Vec3::new(v.y, v.x, 0.0);
                let mid_pt = mid_rotation * start_pt;
                points.push((
                    transform.transform_point(mid_pt),
                    "Midpoint".to_string(),
                    70,
                ));
            }
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::pdms_shape::BrepShapeTrait;

    fn export_mesh_to_obj(mesh: &PlantMesh, filename: &str) {
        use std::fs;
        use std::io::Write;

        let output_dir = "test_output/revolution";
        fs::create_dir_all(output_dir).ok();
        let path = format!("{}/{}", output_dir, filename);

        let mut file = fs::File::create(&path).expect("无法创建文件");
        writeln!(file, "# Revolution test mesh").ok();

        for (v, n) in mesh.vertices.iter().zip(mesh.normals.iter()) {
            writeln!(file, "v {} {} {}", v.x, v.y, v.z).ok();
            writeln!(file, "vn {} {} {}", n.x, n.y, n.z).ok();
        }

        for chunk in mesh.indices.chunks(3) {
            if chunk.len() == 3 {
                writeln!(file, "f {} {} {}", chunk[0] + 1, chunk[1] + 1, chunk[2] + 1).ok();
            }
        }
        println!("   📄 已导出: {}", path);
    }

    /// 测试: 实际案例 24381_36946 - 带 FRAD 圆角
    /// 
    /// 原始数据:
    /// [{ FRAD: 0, x: 38864, y: 23400 }, { FRAD: 0, x: 15464, y: 23400 },
    ///  { FRAD: 23400, x: 38864, y: 23400 }, { FRAD: 0, x: 38864, y: 0 }]
    /// 
    /// 在 PDMS REVO 中：
    /// - x = 沿旋转轴的位置（高度）
    /// - y = 径向距离
    /// - FRAD = 圆角半径
    /// - 默认绕 X 轴旋转 360°
    #[test]
    fn test_revolution_case_24381_36946_with_frad() {
        use crate::prim_geo::profile_processor::ProfileProcessor;

        // 原始数据：Vec3(x, y, FRAD)
        // x = 高度，y = 径向距离，z = FRAD 圆角半径
        let vertices = vec![
            Vec3::new(38864.0, 23400.0, 0.0),     // FRAD=0
            Vec3::new(15464.0, 23400.0, 0.0),     // FRAD=0
            Vec3::new(38864.0, 23400.0, 23400.0), // FRAD=23400 (圆角)
            Vec3::new(38864.0, 0.0, 0.0),         // FRAD=0, 在轴上
        ];

        println!("📊 案例 24381_36946 带 FRAD 圆角:");
        println!("   原始数据 (x=高度, y=径向, z=FRAD):");
        for (i, v) in vertices.iter().enumerate() {
            println!("   点{}: x={}, y={}, FRAD={}", i, v.x, v.y, v.z);
        }

        // 使用 ProfileProcessor 处理 FRAD 圆角
        let processor = ProfileProcessor::new_single(vertices.clone());
        let profile = processor.process("case_24381_36946", Some("24381_36946"));
        
        match profile {
            Ok(processed) => {
                println!("   FRAD处理后轮廓点数: {}", processed.contour_points.len());

                // 将处理后的轮廓转换为 Revolution 的 verts 格式
                // ProfileProcessor 输出: (x=原x, y=原y)
                // Revolution.verts: Vec3(x, y, 0) 其中 x=高度, y=径向
                let processed_verts: Vec<Vec3> = processed.contour_points.iter()
                    .map(|p| Vec3::new(p.x, p.y, 0.0))
                    .collect();

                println!("   处理后顶点:");
                for (i, v) in processed_verts.iter().enumerate() {
                    println!("     点{}: x(高度)={:.1}, y(径向)={:.1}", i, v.x, v.y);
                }

                // 创建 Revolution
                // PDMS 格式：x=轴向(高度), y=径向
                let revolution = Revolution {
                    verts: vec![processed_verts],
                    angle: 360.0,
                };

                // 生成网格
                if let Some(mesh) = revolution.gen_csg_mesh() {
                    // libgm 内部绕 Y 轴生成，轴上顶点是 x=0 且 z=0 的点
                    let axis_points: Vec<_> = mesh.vertices.iter()
                        .filter(|v| (v.x * v.x + v.z * v.z).sqrt() < 1.0)
                        .collect();
                    
                    export_mesh_to_obj(&mesh, "case_24381_36946_with_frad.obj");
                    println!("   顶点数: {}", mesh.vertices.len());
                    println!("   三角形数: {}", mesh.indices.len() / 3);
                    println!("   轴上顶点数: {}", axis_points.len());
                } else {
                    println!("⚠️ Revolution::gen_csg_mesh 返回 None");
                }
            }
            Err(e) => {
                println!("⚠️ ProfileProcessor.process 失败: {}", e);
            }
        }

        println!("✅ 案例 24381_36946 测试完成");
    }

    /// 测试: 简单圆柱（无圆角）
    #[test]
    fn test_revolution_simple_cylinder() {
        // 简单圆柱：半径50，高度100
        // PDMS 格式：x=轴向(高度), y=径向
        let revolution = Revolution {
            verts: vec![vec![
                Vec3::new(0.0, 50.0, 0.0),   // 底部外边缘 (轴向=0, 径向=50)
                Vec3::new(100.0, 50.0, 0.0), // 顶部外边缘 (轴向=100, 径向=50)
                Vec3::new(100.0, 0.0, 0.0),  // 顶部轴上 (轴向=100, 径向=0)
                Vec3::new(0.0, 0.0, 0.0),    // 底部轴上 (轴向=0, 径向=0)
            ]],
            angle: 360.0,
        };

        println!("📊 简单圆柱测试:");
        if let Some(mesh) = revolution.gen_csg_mesh() {
            // libgm 内部绕 Y 轴生成，轴上顶点是 x=0 且 z=0 的点
            let axis_points: Vec<_> = mesh.vertices.iter()
                .filter(|v| (v.x * v.x + v.z * v.z).sqrt() < 1.0)
                .collect();
            
            export_mesh_to_obj(&mesh, "simple_cylinder.obj");
            println!("   顶点数: {}", mesh.vertices.len());
            println!("   三角形数: {}", mesh.indices.len() / 3);
            println!("   轴上顶点数: {} (预期>=2)", axis_points.len());
            // 轴上至少有2个共享顶点（顶部和底部），可能因为轮廓闭合有更多
            assert!(axis_points.len() >= 2, "应至少有2个轴上共享顶点");
        } else {
            panic!("Revolution::gen_csg_mesh 返回 None");
        }
        println!("✅ 简单圆柱测试通过");
    }

    /// 测试: 圆锥（顶点在轴上）
    #[test]
    fn test_revolution_cone() {
        // 圆锥：底部半径80，顶点在轴上
        // PDMS 格式：x=轴向(高度), y=径向
        let revolution = Revolution {
            verts: vec![vec![
                Vec3::new(0.0, 80.0, 0.0),   // 底部外边缘 (轴向=0, 径向=80)
                Vec3::new(150.0, 0.0, 0.0),  // 顶点（在轴上）(轴向=150, 径向=0)
                Vec3::new(0.0, 0.0, 0.0),    // 底部轴上 (轴向=0, 径向=0)
            ]],
            angle: 360.0,
        };

        println!("📊 圆锥测试:");
        if let Some(mesh) = revolution.gen_csg_mesh() {
            // libgm 内部绕 Y 轴生成，轴上顶点是 x=0 且 z=0 的点
            let axis_points: Vec<_> = mesh.vertices.iter()
                .filter(|v| (v.x * v.x + v.z * v.z).sqrt() < 1.0)
                .collect();
            
            export_mesh_to_obj(&mesh, "cone.obj");
            println!("   顶点数: {}", mesh.vertices.len());
            println!("   三角形数: {}", mesh.indices.len() / 3);
            println!("   轴上顶点数: {} (预期2)", axis_points.len());
        } else {
            panic!("Revolution::gen_csg_mesh 返回 None");
        }
        println!("✅ 圆锥测试通过");
    }

    /// 测试: 半球（带圆弧轮廓）
    #[test]
    fn test_revolution_hemisphere_with_frad() {
        use crate::prim_geo::profile_processor::ProfileProcessor;

        // 半球：使用 FRAD 生成圆弧
        // 三个点形成直角，FRAD 在角点处生成 1/4 圆弧
        let radius = 50.0f32;
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),           // 底部中心（轴上）
            Vec3::new(0.0, radius, radius),     // 角点，带圆角
            Vec3::new(radius, 0.0, 0.0),        // 顶部（轴上）
        ];

        println!("📊 半球测试 (FRAD 圆弧):");
        let processor = ProfileProcessor::new_single(vertices);
        
        match processor.process("hemisphere", Some("hemisphere")) {
            Ok(processed) => {
                println!("   处理后轮廓点数: {}", processed.contour_points.len());

                let processed_verts: Vec<Vec3> = processed.contour_points.iter()
                    .map(|p| Vec3::new(p.x, p.y, 0.0))
                    .collect();

                let revolution = Revolution {
                    verts: vec![processed_verts],
                    angle: 360.0,
                };

                if let Some(mesh) = revolution.gen_csg_mesh() {
                    export_mesh_to_obj(&mesh, "hemisphere_with_frad.obj");
                    println!("   顶点数: {}", mesh.vertices.len());
                    println!("   三角形数: {}", mesh.indices.len() / 3);
                } else {
                    println!("⚠️ Revolution::gen_csg_mesh 返回 None");
                }
            }
            Err(e) => {
                println!("⚠️ ProfileProcessor.process 失败: {} (可能FRAD参数不合适)", e);
            }
        }
        println!("✅ 半球测试完成");
    }
}
