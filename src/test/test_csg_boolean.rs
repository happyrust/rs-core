//! 测试 CSG 布尔运算
//!
//! 使用墙体拉伸体与不同基本体进行布尔运算测试

use std::path::Path;

use glam::{DMat4, Vec2, Vec3};

use crate::csg::manifold::{ManifoldCrossSectionRust, ManifoldOpType, ManifoldRust};
use crate::fast_model::export_model::export_glb::export_single_mesh_to_glb;
use crate::geometry::csg::{unit_box_mesh, unit_cylinder_mesh, unit_sphere_mesh};
use crate::mesh_precision::LodMeshSettings;

/// 输出目录
const OUTPUT_DIR: &str = "test_output/csg_boolean";

/// 确保输出目录存在
fn ensure_output_dir() {
    std::fs::create_dir_all(OUTPUT_DIR).expect("创建输出目录失败");
}

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

    // 变换圆柱体：旋转90度使轴向从Z变为Y，然后缩放和平移
    // unit_cylinder Z范围[0,1]，旋转后Y范围[0,-1]，缩放0.5后Y范围[0,-0.5]
    // 平移Y=0.25使圆柱体居中于Y=0，即Y范围[-0.25, 0.25]
    let scale = DMat4::from_scale(glam::DVec3::new(0.2, 0.2, 0.5));
    let rotate = DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2);
    let translate = DMat4::from_translation(glam::DVec3::new(0.0, 0.25, 1.5));
    // 变换顺序：先缩放，再旋转，最后平移 (从右到左读)
    let transform = translate * rotate * scale;

    let cylinder = ManifoldRust::import_glb_to_manifold(&cyl_path, transform, false)
        .expect("导入圆柱体失败");
    let _ = std::fs::remove_file(&cyl_path);

    let cyl_mesh = cylinder.get_mesh();
    println!("圆柱体: {} 顶点, {} 三角形",
        cyl_mesh.vertices.len() / 3,
        cyl_mesh.indices.len() / 3);
    if let Some(aabb) = cyl_mesh.cal_aabb() {
        println!("圆柱体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z,
            aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
    }

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[cylinder]);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_cylinder.glb");
    result.export_to_glb(&output_path).expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 墙体 - 圆柱体 测试通过");
}

#[test]
fn test_wall_subtract_sphere() {
    println!("\n=== 测试: 墙体 - 球体 ===");

    // 创建墙体
    let wall_section = create_wall_section(2.0, 0.3);
    let wall = wall_section.extrude(3.0, 1);

    let wall_mesh = wall.get_mesh();
    println!("墙体: {} 顶点, {} 三角形",
        wall_mesh.vertices.len() / 3,
        wall_mesh.indices.len() / 3);
    if let Some(aabb) = wall_mesh.cal_aabb() {
        println!("墙体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z,
            aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
    }

    // 创建球体
    let sphere_mesh = unit_sphere_mesh();

    let temp_dir = std::env::temp_dir();
    let sphere_path = temp_dir.join("test_sphere.glb");
    export_single_mesh_to_glb(&sphere_mesh, &sphere_path).expect("导出球体失败");

    // 缩放: 半径0.5 (直径1.0 > 墙体厚度0.3), 位于墙体中心
    // 球体需要穿透墙体才能产生有效的布尔减法效果
    let transform = DMat4::from_scale_rotation_translation(
        glam::DVec3::new(0.5, 0.5, 0.5),
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
    if let Some(aabb) = sphere_mesh_out.cal_aabb() {
        println!("球体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z,
            aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
    }

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[sphere]);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_sphere.glb");
    result.export_to_glb(&output_path).expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 墙体 - 球体 测试通过");
}

#[test]
fn test_wall_subtract_box() {
    println!("\n=== 测试: 墙体 - 盒子 ===");

    // 创建墙体
    let wall_section = create_wall_section(2.0, 0.3);
    let wall = wall_section.extrude(3.0, 1);

    let wall_mesh = wall.get_mesh();
    println!("墙体: {} 顶点, {} 三角形",
        wall_mesh.vertices.len() / 3,
        wall_mesh.indices.len() / 3);
    if let Some(aabb) = wall_mesh.cal_aabb() {
        println!("墙体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z,
            aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
    }

    // 创建盒子
    let box_mesh = unit_box_mesh();

    let temp_dir = std::env::temp_dir();
    let box_path = temp_dir.join("test_box.glb");
    export_single_mesh_to_glb(&box_mesh, &box_path).expect("导出盒子失败");

    // 缩放: 0.5x0.5x0.5 (边长1.0 > 墙体厚度0.3), 位于墙体中心
    let transform = DMat4::from_scale_rotation_translation(
        glam::DVec3::new(0.5, 0.5, 0.5),
        glam::DQuat::IDENTITY,
        glam::DVec3::new(0.0, 0.0, 1.5),
    );

    let box_manifold = ManifoldRust::import_glb_to_manifold(&box_path, transform, false)
        .expect("导入盒子失败");
    let _ = std::fs::remove_file(&box_path);

    let box_mesh_out = box_manifold.get_mesh();
    println!("盒子: {} 顶点, {} 三角形",
        box_mesh_out.vertices.len() / 3,
        box_mesh_out.indices.len() / 3);
    if let Some(aabb) = box_mesh_out.cal_aabb() {
        println!("盒子 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z,
            aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
    }

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[box_manifold]);
    let result_mesh = result.get_mesh();

    println!("结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3);

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_box.glb");
    result.export_to_glb(&output_path).expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

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
        // 变换圆柱体：旋转90度使轴向从Z变为Y，然后缩放和平移
        let scale = DMat4::from_scale(glam::DVec3::new(0.15, 0.15, 0.5));
        let rotate = DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2);
        let translate = DMat4::from_translation(glam::DVec3::new(0.0, 0.25, z_pos));
        let transform = translate * rotate * scale;

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

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_multiple.glb");
    result.export_to_glb(&output_path).expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 墙体 - 多个基本体 测试通过");
}
