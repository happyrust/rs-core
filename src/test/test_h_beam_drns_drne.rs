use crate::RefnoEnum;
use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::spine::{Line3D, SweepPath3D};
/// 测试 H 型钢的 drns/drne 端面方向控制
///
/// 创建一个H型钢，两端端面都是45度斜切
use crate::prim_geo::sweep_solid::SweepSolid;
use glam::{DVec3, Vec2, Vec3};
use std::f32::consts::PI;

/// 生成H型钢轮廓点
///
/// H型钢标准尺寸：
/// - 总高度 (H): 200mm
/// - 翼缘宽度 (B): 200mm  
/// - 腹板厚度 (t1): 8mm
/// - 翼缘厚度 (t2): 12mm
///
/// 轮廓沿着H型钢的外围走一圈（包括内凹部分）
fn create_h_beam_profile() -> Vec<Vec2> {
    let h = 200.0; // 总高度
    let b = 200.0; // 翼缘宽度
    let t1 = 8.0; // 腹板厚度
    let t2 = 12.0; // 翼缘厚度

    let half_h = h / 2.0;
    let half_b = b / 2.0;
    let half_t1 = t1 / 2.0;

    // H型钢轮廓（逆时针方向，从左下角外侧开始，沿外围走一圈）
    vec![
        // 1. 左下翼缘 - 底部外侧
        Vec2::new(-half_b, -half_h),
        // 2. 左下翼缘 - 左侧外侧
        Vec2::new(-half_b, -half_h + t2),
        // 3. 进入腹板 - 内凹
        Vec2::new(-half_t1, -half_h + t2),
        // 4. 腹板左侧向上
        Vec2::new(-half_t1, half_h - t2),
        // 5. 进入左上翼缘 - 内凹
        Vec2::new(-half_b, half_h - t2),
        // 6. 左上翼缘 - 左侧外侧
        Vec2::new(-half_b, half_h),
        // 7. 左上翼缘 - 顶部
        Vec2::new(half_b, half_h),
        // 8. 右上翼缘 - 右侧外侧
        Vec2::new(half_b, half_h - t2),
        // 9. 退出右上翼缘 - 内凸
        Vec2::new(half_t1, half_h - t2),
        // 10. 腹板右侧向下
        Vec2::new(half_t1, -half_h + t2),
        // 11. 退出右下翼缘 - 内凸
        Vec2::new(half_b, -half_h + t2),
        // 12. 右下翼缘 - 底部外侧
        Vec2::new(half_b, -half_h),
        // 自动闭合到第一个点
    ]
}

#[test]
fn test_h_beam_with_45_degree_end_faces() {
    println!("\n=== 测试H型钢45度斜切端面 ===");

    // 创建H型钢截面
    let h_beam_points = create_h_beam_profile();
    println!("  H型钢截面: 200x200mm, 12点轮廓");

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: h_beam_points.clone(),
        frads: vec![0.0; h_beam_points.len()], // 无圆角
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    });

    // 创建1000mm长的直线路径（沿Z轴）
    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 1000.0,
        is_spine: true,
    });

    // 计算45度斜切的端面方向
    // 起始端面：向后倾斜45度（相对于Z轴）
    let drns_45 = DVec3::new(0.0, 0.0, 1.0).normalize() + DVec3::new(0.0, 1.0, 0.0).normalize();
    let drns = drns_45.normalize();

    // 结束端面：向前倾斜45度
    let drne_45 = DVec3::new(0.0, 0.0, 1.0).normalize() + DVec3::new(0.0, -1.0, 0.0).normalize();
    let drne = drne_45.normalize();

    println!(
        "  起始端面方向 (drns): [{:.3}, {:.3}, {:.3}] (45°倾斜)",
        drns.x, drns.y, drns.z
    );
    println!(
        "  结束端面方向 (drne): [{:.3}, {:.3}, {:.3}] (45°倾斜)",
        drne.x, drne.y, drne.z
    );

    let sweep_solid = SweepSolid {
        profile,
        drns: Some(drns),
        drne: Some(drne),
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: 1000.0,
        path: line_path.clone(),
        lmirror: false,
    };

    // 生成 CSG mesh
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("\n  ✅ CSG Mesh 生成成功！");
            println!("  网格统计:");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    法线数: {}", csg_mesh.normals.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/h_beam_45degree_ends.obj") {
                println!("    ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("    ✅ OBJ文件导出成功: test_output/h_beam_45degree_ends.obj");
                println!("    📐 可以在Blender/MeshLab中查看45度斜切效果");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Shape生成失败: {}", e);
            panic!("H型钢mesh生成失败");
        }
    }

    println!("\n✅ H型钢45度斜切端面测试通过");
}

#[test]
fn test_h_beam_different_end_angles() {
    println!("\n=== 测试H型钢不同端面角度 ===");

    // 创建H型钢截面
    let h_beam_points = create_h_beam_profile();

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: h_beam_points.clone(),
        frads: vec![0.0; h_beam_points.len()], // 无圆角
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    });

    // 创建800mm长的直线路径
    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 800.0,
        is_spine: true,
    });

    // 起始端面：30度倾斜
    let angle_30 = 30.0_f64.to_radians();
    let drns = DVec3::new(0.0, angle_30.sin(), angle_30.cos()).normalize();

    // 结束端面：60度倾斜
    let angle_60 = 60.0_f64.to_radians();
    let drne = DVec3::new(0.0, -angle_60.sin(), angle_60.cos()).normalize();

    println!("  起始端面: 30度倾斜");
    println!("  结束端面: 60度倾斜");

    let sweep_solid = SweepSolid {
        profile,
        drns: Some(drns),
        drne: Some(drne),
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: 800.0,
        path: line_path,
        lmirror: false,
    };

    // 生成 CSG mesh
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("  顶点数: {}", csg_mesh.vertices.len());
            println!("  三角形数: {}", csg_mesh.indices.len() / 3);

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/h_beam_30_60_degree_ends.obj") {
                println!("  ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("  ✅ OBJ文件导出成功: test_output/h_beam_30_60_degree_ends.obj");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Shape生成失败: {}", e);
            panic!("H型钢不同角度mesh生成失败");
        }
    }

    println!("✅ H型钢不同端面角度测试通过");
}

#[test]
fn test_h_beam_normal_ends() {
    println!("\n=== 测试H型钢垂直端面（对照组）===");

    // 创建H型钢截面
    let h_beam_points = create_h_beam_profile();

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: h_beam_points.clone(),
        frads: vec![0.0; h_beam_points.len()], // 无圆角
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    });

    // 创建1000mm长的直线路径
    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 1000.0,
        is_spine: true,
    });

    // 不设置 drns/drne，使用默认垂直端面
    let sweep_solid = SweepSolid {
        profile,
        drns: None, // 默认垂直端面
        drne: None, // 默认垂直端面
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: 1000.0,
        path: line_path,
        lmirror: false,
    };

    println!("  端面方向: 默认（垂直于路径）");

    // 生成 CSG mesh
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("  顶点数: {}", csg_mesh.vertices.len());

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/h_beam_normal_ends.obj") {
                println!("  ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("  ✅ OBJ文件导出成功: test_output/h_beam_normal_ends.obj");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Shape生成失败: {}", e);
            panic!("H型钢默认端面mesh生成失败");
        }
    }

    println!("✅ H型钢垂直端面测试通过");
}
