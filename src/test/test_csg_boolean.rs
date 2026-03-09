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

/// 计算三角形面积（从 3 个顶点坐标）
fn triangle_area(v0: Vec3, v1: Vec3, v2: Vec3) -> f32 {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    e1.cross(e2).length() * 0.5
}

/// 统计薄片三角形（面积 < threshold 的比例）
/// 返回 (薄片数, 总三角形数, 薄片比例)
fn count_slivers(
    vertices: &[f32],
    indices: &[u32],
    area_threshold: f32,
) -> (usize, usize, f64) {
    let tri_count = indices.len() / 3;
    if tri_count == 0 {
        return (0, 0, 0.0);
    }
    let mut sliver_count = 0;
    for t in 0..tri_count {
        let i0 = indices[t * 3] as usize;
        let i1 = indices[t * 3 + 1] as usize;
        let i2 = indices[t * 3 + 2] as usize;
        let v0 = Vec3::new(vertices[i0 * 3], vertices[i0 * 3 + 1], vertices[i0 * 3 + 2]);
        let v1 = Vec3::new(vertices[i1 * 3], vertices[i1 * 3 + 1], vertices[i1 * 3 + 2]);
        let v2 = Vec3::new(vertices[i2 * 3], vertices[i2 * 3 + 1], vertices[i2 * 3 + 2]);
        let area = triangle_area(v0, v1, v2);
        if area < area_threshold {
            sliver_count += 1;
        }
    }
    let ratio = sliver_count as f64 / tri_count as f64;
    (sliver_count, tri_count, ratio)
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
    println!(
        "墙体: {} 顶点, {} 三角形",
        wall_mesh.vertices.len() / 3,
        wall_mesh.indices.len() / 3
    );

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

    let cylinder =
        ManifoldRust::import_glb_to_manifold(&cyl_path, transform, false).expect("导入圆柱体失败");
    let _ = std::fs::remove_file(&cyl_path);

    let cyl_mesh = cylinder.get_mesh();
    println!(
        "圆柱体: {} 顶点, {} 三角形",
        cyl_mesh.vertices.len() / 3,
        cyl_mesh.indices.len() / 3
    );
    if let Some(aabb) = cyl_mesh.cal_aabb() {
        println!(
            "圆柱体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z, aabb.maxs.x, aabb.maxs.y, aabb.maxs.z
        );
    }

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[cylinder]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_cylinder.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
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
    println!(
        "墙体: {} 顶点, {} 三角形",
        wall_mesh.vertices.len() / 3,
        wall_mesh.indices.len() / 3
    );
    if let Some(aabb) = wall_mesh.cal_aabb() {
        println!(
            "墙体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z, aabb.maxs.x, aabb.maxs.y, aabb.maxs.z
        );
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

    let sphere =
        ManifoldRust::import_glb_to_manifold(&sphere_path, transform, false).expect("导入球体失败");
    let _ = std::fs::remove_file(&sphere_path);

    let sphere_mesh_out = sphere.get_mesh();
    println!(
        "球体: {} 顶点, {} 三角形",
        sphere_mesh_out.vertices.len() / 3,
        sphere_mesh_out.indices.len() / 3
    );
    if let Some(aabb) = sphere_mesh_out.cal_aabb() {
        println!(
            "球体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z, aabb.maxs.x, aabb.maxs.y, aabb.maxs.z
        );
    }

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[sphere]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_sphere.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
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
    println!(
        "墙体: {} 顶点, {} 三角形",
        wall_mesh.vertices.len() / 3,
        wall_mesh.indices.len() / 3
    );
    if let Some(aabb) = wall_mesh.cal_aabb() {
        println!(
            "墙体 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z, aabb.maxs.x, aabb.maxs.y, aabb.maxs.z
        );
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

    let box_manifold =
        ManifoldRust::import_glb_to_manifold(&box_path, transform, false).expect("导入盒子失败");
    let _ = std::fs::remove_file(&box_path);

    let box_mesh_out = box_manifold.get_mesh();
    println!(
        "盒子: {} 顶点, {} 三角形",
        box_mesh_out.vertices.len() / 3,
        box_mesh_out.indices.len() / 3
    );
    if let Some(aabb) = box_mesh_out.cal_aabb() {
        println!(
            "盒子 AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            aabb.mins.x, aabb.mins.y, aabb.mins.z, aabb.maxs.x, aabb.maxs.y, aabb.maxs.z
        );
    }

    // 布尔减法
    let result = wall.batch_boolean_subtract(&[box_manifold]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_box.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
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

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 导出结果到 GLB
    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("wall_subtract_multiple.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 墙体 - 多个基本体 测试通过");
}

// ==================== Phase 3: 原生 Manifold 构造 + 共面布尔回归测试 ====================

#[test]
fn test_native_box_subtract_box_coplanar() {
    println!("\n=== 测试: 原生 BOX-BOX 共面布尔 ===");

    // 正实体：1x1x1 的盒子，中心在原点
    let pos = ManifoldRust::native_box(1.0, 1.0, 1.0);

    // 负实体：同样 1x1x1，平移使其与正实体共享一个面（x=0.5）
    // 负实体中心在 (1.0, 0.0, 0.0)，左面在 x=0.5 恰好与正实体右面重合
    let neg_transform = DMat4::from_translation(glam::DVec3::new(1.0, 0.0, 0.0));
    let neg = ManifoldRust::native_box(1.0, 1.0, 1.0).apply_transform(neg_transform);

    let pos_mesh = pos.get_mesh();
    let neg_mesh = neg.get_mesh();
    println!(
        "正实体: {} 顶点, {} 三角形",
        pos_mesh.vertices.len() / 3,
        pos_mesh.indices.len() / 3
    );
    println!(
        "负实体: {} 顶点, {} 三角形",
        neg_mesh.vertices.len() / 3,
        neg_mesh.indices.len() / 3
    );

    let result = pos.batch_boolean_subtract(&[neg]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    // 薄片检测：面积 < 0.001 的三角形占比应低于 5%
    let (slivers, total, ratio) = count_slivers(&result_mesh.vertices, &result_mesh.indices, 0.001);
    println!(
        "薄片统计: {}/{} ({:.2}%)",
        slivers,
        total,
        ratio * 100.0
    );
    assert!(
        ratio < 0.05,
        "共面 BOX-BOX 薄片比例 {:.2}% 超过 5% 阈值",
        ratio * 100.0
    );

    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("native_box_sub_box_coplanar.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 原生 BOX-BOX 共面布尔 测试通过");
}

#[test]
fn test_native_box_subtract_box_flush_face() {
    println!("\n=== 测试: 原生 BOX-BOX 完全贴合面 ===");

    // 最困难的情况：两个盒子完全共享一整面
    // 正实体：2x1x1，中心在原点
    let pos = ManifoldRust::native_box(2.0, 1.0, 1.0);

    // 负实体：1x1x1，中心在 (0.5, 0, 0)
    // 负实体右面在 x=1.0（正实体右面），左面在 x=0.0（正实体中心）
    // y/z 完全对齐 → 4条边共面
    let neg_transform = DMat4::from_translation(glam::DVec3::new(0.5, 0.0, 0.0));
    let neg = ManifoldRust::native_box(1.0, 1.0, 1.0).apply_transform(neg_transform);

    let result = pos.batch_boolean_subtract(&[neg]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    let (slivers, total, ratio) = count_slivers(&result_mesh.vertices, &result_mesh.indices, 0.001);
    println!(
        "薄片统计: {}/{} ({:.2}%)",
        slivers,
        total,
        ratio * 100.0
    );
    assert!(
        ratio < 0.05,
        "完全贴合面 BOX-BOX 薄片比例 {:.2}% 超过 5% 阈值",
        ratio * 100.0
    );

    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("native_box_sub_box_flush.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 原生 BOX-BOX 完全贴合面 测试通过");
}

#[test]
fn test_native_cylinder_subtract_cylinder_coplanar() {
    println!("\n=== 测试: 原生 CYL-CYL 共面布尔 ===");

    // 正实体：半径 0.5，高度 2.0，底面 z=0
    let pos = ManifoldRust::native_cylinder(0.5, 2.0, 64);

    // 负实体：同样尺寸，Z轴对齐但平移到顶端
    // 底面在 z=2.0（正实体顶面），共享一整个圆形面
    let neg_transform = DMat4::from_translation(glam::DVec3::new(0.0, 0.0, 2.0));
    let neg = ManifoldRust::native_cylinder(0.5, 2.0, 64).apply_transform(neg_transform);

    let pos_mesh = pos.get_mesh();
    let neg_mesh = neg.get_mesh();
    println!(
        "正实体: {} 顶点, {} 三角形",
        pos_mesh.vertices.len() / 3,
        pos_mesh.indices.len() / 3
    );
    println!(
        "负实体: {} 顶点, {} 三角形",
        neg_mesh.vertices.len() / 3,
        neg_mesh.indices.len() / 3
    );

    let result = pos.batch_boolean_subtract(&[neg]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    let (slivers, total, ratio) = count_slivers(&result_mesh.vertices, &result_mesh.indices, 0.001);
    println!(
        "薄片统计: {}/{} ({:.2}%)",
        slivers,
        total,
        ratio * 100.0
    );
    assert!(
        ratio < 0.05,
        "共面 CYL-CYL 薄片比例 {:.2}% 超过 5% 阈值",
        ratio * 100.0
    );

    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("native_cyl_sub_cyl_coplanar.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 原生 CYL-CYL 共面布尔 测试通过");
}

#[test]
fn test_native_sphere_subtract_box() {
    println!("\n=== 测试: 原生 SPHERE-BOX 布尔 ===");

    // 正实体：半径 1.0 的球体，中心在原点
    let pos = ManifoldRust::native_sphere(1.0, 64);

    // 负实体：1x1x1 的盒子，中心在 (0.5, 0, 0)
    // 盒子右半部分在球体内，产生弧面切割
    let neg_transform = DMat4::from_translation(glam::DVec3::new(0.5, 0.0, 0.0));
    let neg = ManifoldRust::native_box(1.0, 1.0, 1.0).apply_transform(neg_transform);

    let pos_mesh = pos.get_mesh();
    let neg_mesh = neg.get_mesh();
    println!(
        "正实体: {} 顶点, {} 三角形",
        pos_mesh.vertices.len() / 3,
        pos_mesh.indices.len() / 3
    );
    println!(
        "负实体: {} 顶点, {} 三角形",
        neg_mesh.vertices.len() / 3,
        neg_mesh.indices.len() / 3
    );

    let result = pos.batch_boolean_subtract(&[neg]);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    let (slivers, total, ratio) = count_slivers(&result_mesh.vertices, &result_mesh.indices, 0.001);
    println!(
        "薄片统计: {}/{} ({:.2}%)",
        slivers,
        total,
        ratio * 100.0
    );
    assert!(
        ratio < 0.10,
        "SPHERE-BOX 薄片比例 {:.2}% 超过 10% 阈值",
        ratio * 100.0
    );

    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("native_sphere_sub_box.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 原生 SPHERE-BOX 布尔 测试通过");
}

#[test]
fn test_native_box_subtract_multiple_cylinders() {
    println!("\n=== 测试: 原生 BOX - 多个圆柱体穿孔 ===");

    // 正实体：大盒子 10x10x1（类似墙板）
    let pos = ManifoldRust::native_box(10.0, 10.0, 1.0);

    // 负实体：3 个圆柱体沿 Z 轴穿透墙板
    let mut negs = Vec::new();
    for i in 0..3 {
        let x_pos = -3.0 + i as f64 * 3.0;
        let transform = DMat4::from_translation(glam::DVec3::new(x_pos, 0.0, -1.0));
        let cyl = ManifoldRust::native_cylinder(0.5, 3.0, 64).apply_transform(transform);
        negs.push(cyl);
    }

    println!("创建了 {} 个孔洞", negs.len());

    let result = pos.batch_boolean_subtract(&negs);
    let result_mesh = result.get_mesh();

    println!(
        "结果: {} 顶点, {} 三角形",
        result_mesh.vertices.len() / 3,
        result_mesh.indices.len() / 3
    );

    assert!(result_mesh.indices.len() > 0, "布尔运算结果不应为空");

    let (slivers, total, ratio) = count_slivers(&result_mesh.vertices, &result_mesh.indices, 0.001);
    println!(
        "薄片统计: {}/{} ({:.2}%)",
        slivers,
        total,
        ratio * 100.0
    );
    assert!(
        ratio < 0.10,
        "多圆柱穿孔薄片比例 {:.2}% 超过 10% 阈值",
        ratio * 100.0
    );

    ensure_output_dir();
    let output_path = Path::new(OUTPUT_DIR).join("native_box_sub_multi_cyl.glb");
    result
        .export_to_glb(&output_path)
        .expect("导出布尔运算结果失败");
    println!("已导出: {:?}", output_path);

    println!("✅ 原生 BOX - 多圆柱穿孔 测试通过");
}
