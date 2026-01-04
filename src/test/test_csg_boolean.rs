//! 测试 CSG 布尔运算
//!
//! 使用墙体拉伸体与不同基本体进行布尔运算测试

use glam::{DMat4, Vec2, Vec3};

use crate::csg::manifold::{ManifoldCrossSectionRust, ManifoldOpType, ManifoldRust};
use crate::fast_model::export_model::export_glb::export_single_mesh_to_glb;
use crate::geometry::csg::{generate_csg_mesh, unit_box_mesh, unit_cylinder_mesh, unit_sphere_mesh};
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::{RTorus, Sphere};

/// 创建墙体截面 (矩形)
fn create_wall_section(width: f32, thickness: f32) -> ManifoldCrossSectionRust {
    let hw = width / 2.0;
    let ht = thickness / 2.0;
    let pts = vec![
        Vec2::new(-hw, -ht),
        Vec2::new(hw, -ht),
        Vec2::new(hw, ht),
        Vec2::new(-hw, ht),
    ];
    ManifoldCrossSectionRust::from_points(&pts)
}

/// 创建 L 形墙体截面
fn create_l_wall_section(width: f32, depth: f32, thickness: f32) -> ManifoldCrossSectionRust {
    let pts = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(width, 0.0),
        Vec2::new(width, thickness),
        Vec2::new(thickness, thickness),
        Vec2::new(thickness, depth),
        Vec2::new(0.0, depth),
    ];
    ManifoldCrossSectionRust::from_points(&pts)
}

#[test]
fn test_wall_subtract_cylinder() {
    println!("\n=== 测试: 墙体 - 圆柱体 ===");

    // 创建墙体拉伸体
    let wall_section = create_wall_section(2.0, 0.3);
    let wall = wall_section.extrude(3.0, 1);

    let wall_mesh = wall.get_mesh();
    println!("墙体: {} 顶点, {} 三角形",
        wall_mesh.vertices.len() / 3,
        wall_mesh.indices.len() / 3);

    // 创建圆柱体 (使用 unit_cylinder 然后缩放)
    let settings = LodMeshSettings::default();
    let cyl_mesh = unit_cylinder_mesh(&settings, false);

    // 导出为 GLB 然后导入为 Manifold
    let temp_dir = std::env::temp_dir();
    let cyl_path = temp_dir.join("test_cyl.glb");
    export_single_mesh_to_glb(&cyl_mesh, &cyl_path).expect("导出圆柱体失败");

    // 缩放和平移变换: 半径0.2, 高度0.5, 位于墙体中心
    let transform = DMat4::from_scale_rotation_translation(
        glam::DVec3::new(0.2, 0.2, 0.5),
        glam::DQuat::IDENTITY,
        glam::DVec3::new(0.0, 0.0, 1.5),
    );

    let cylinder = ManifoldRust::import_glb_to_manifold(&cyl_path, transform, false)
        .expect("导入圆柱体失败");
    let _ = std::fs::remove_file(&cyl_path);

    let cyl_mesh = cylinder.get_mesh();
    println!("圆柱体: {} 顶点, {} 三角形",
        cyl_mesh.vertices.len() / 3,
        cyl_mesh.indices.len() / 3);

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[cylinder]);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");
    println!("✅ 墙体 - 圆柱体 测试通过");
}

#[test]
fn test_wall_subtract_sphere() {
    println!("\n=== 测试: 墙体 - 球体 ===");

    // 创建墙体
    let wall_section = create_wall_section(2.0, 0.3);
    let wall = wall_section.extrude(3.0, 1);

    // 创建球体
    let settings = LodMeshSettings::default();
    let sphere_mesh = unit_sphere_mesh();

    let temp_dir = std::env::temp_dir();
    let sphere_path = temp_dir.join("test_sphere.glb");
    export_single_mesh_to_glb(&sphere_mesh, &sphere_path).expect("导出球体失败");

    // 缩放: 半径0.3, 位于墙体中心
    let transform = DMat4::from_scale_rotation_translation(
        glam::DVec3::new(0.3, 0.3, 0.3),
        glam::DQuat::IDENTITY,
        glam::DVec3::new(0.0, 0.0, 1.5),
    );

    let sphere = ManifoldRust::import_glb_to_manifold(&sphere_path, transform, false)
        .expect("导入球体失败");
    let _ = std::fs::remove_file(&sphere_path);

    let sphere_mesh_out = sphere.get_mesh();
    println!("球体: {} 顶点, {} 三角形",
        sphere_mesh_out.vertices.len() / 3,
        sphere_mesh_out.indices.len() / 3);

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[sphere]);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");
    println!("✅ 墙体 - 球体 测试通过");
}

#[test]
fn test_wall_subtract_box() {
    println!("\n=== 测试: 墙体 - 盒子 ===");

    // 创建墙体
    let wall_section = create_wall_section(2.0, 0.3);
    let wall = wall_section.extrude(3.0, 1);

    // 创建盒子
    let box_mesh = unit_box_mesh();

    let temp_dir = std::env::temp_dir();
    let box_path = temp_dir.join("test_box.glb");
    export_single_mesh_to_glb(&box_mesh, &box_path).expect("导出盒子失败");

    // 缩放: 0.4x0.4x0.4, 位于墙体中心
    let transform = DMat4::from_scale_rotation_translation(
        glam::DVec3::new(0.4, 0.4, 0.4),
        glam::DQuat::IDENTITY,
        glam::DVec3::new(0.0, 0.0, 1.5),
    );

    let box_manifold = ManifoldRust::import_glb_to_manifold(&box_path, transform, false)
        .expect("导入盒子失败");
    let _ = std::fs::remove_file(&box_path);

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[box_manifold]);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");
    println!("✅ 墙体 - 盒子 测试通过");
}

#[test]
fn test_wall_subtract_multiple() {
    println!("\n=== 测试: 墙体 - 多个基本体 ===");

    // 创建墙体
    let wall_section = create_wall_section(2.0, 0.3);
    let wall = wall_section.extrude(3.0, 1);

    let temp_dir = std::env::temp_dir();
    let settings = LodMeshSettings::default();

    // 创建多个圆柱体 (模拟管道穿孔)
    let cyl_mesh = unit_cylinder_mesh(&settings, false);
    let mut holes = Vec::new();

    for i in 0..3 {
        let cyl_path = temp_dir.join(format!("test_cyl_{}.glb", i));
        export_single_mesh_to_glb(&cyl_mesh, &cyl_path).expect("导出圆柱体失败");

        let z_pos = 0.5 + i as f64 * 1.0;
        let transform = DMat4::from_scale_rotation_translation(
            glam::DVec3::new(0.15, 0.15, 0.3),
            glam::DQuat::IDENTITY,
            glam::DVec3::new(0.0, 0.0, z_pos),
        );

        let cyl = ManifoldRust::import_glb_to_manifold(&cyl_path, transform, false)
            .expect("导入圆柱体失败");
        let _ = std::fs::remove_file(&cyl_path);
        holes.push(cyl);
    }

    println!("创建了 {} 个孔洞", holes.len());

    // 批量布尔减法
    let result = wall.batch_boolean_subtract(&holes);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");
    println!("✅ 墙体 - 多个基本体 测试通过");
}
