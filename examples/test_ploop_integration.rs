//! 测试 aios-core 与 ploop-rs 的集成
//!
//! 这个示例验证 aios-core 中对 ploop-rs 的使用是否正确
//!
//! 运行方法：
//! ```bash
//! cargo run --example test_ploop_integration
//! ```

use aios_core::prim_geo::wire::{gen_polyline_from_processed_vertices, process_ploop_vertices};
use anyhow::Result;
use glam::Vec3;

fn main() -> Result<()> {
    println!("🧪 测试 aios-core 与 ploop-rs 集成");
    println!("═══════════════════════════════════════════════════\n");

    // 测试 1: 简单矩形（无 FRADIUS）
    test_simple_rectangle()?;

    println!("\n");

    // 测试 2: 带圆角的矩形（有 FRADIUS）
    test_rectangle_with_fradius()?;

    println!("\n");

    // 测试 3: 复杂形状（多个 FRADIUS）
    test_complex_shape()?;

    println!("\n✅ 所有测试通过！");
    Ok(())
}

/// 测试 1: 简单矩形（无 FRADIUS）
fn test_simple_rectangle() -> Result<()> {
    println!("📋 测试 1: 简单矩形（无 FRADIUS）");
    println!("─────────────────────────────────────");

    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0),
    ];

    println!("输入顶点数: {}", vertices.len());

    // 使用 process_ploop_vertices 处理
    let processed = process_ploop_vertices(&vertices, "SIMPLE_RECTANGLE")?;

    println!("处理后顶点数: {}", processed.len());

    // 验证结果
    assert!(processed.len() >= 4, "处理后应该至少有 4 个顶点");

    // 检查是否有 bulge
    let bulge_count = processed
        .iter()
        .filter(|v| v.z.abs() > f32::EPSILON)
        .count();
    println!("bulge 段数: {}", bulge_count);
    assert_eq!(bulge_count, 0, "简单矩形不应该生成圆弧段");

    // 基于处理后的 bulge 顶点生成 Polyline
    let polyline = gen_polyline_from_processed_vertices(&processed)?;
    println!("生成的 Polyline 顶点数: {}", polyline.vertex_data.len());
    println!("Polyline 是否闭合: {}", polyline.is_closed);

    println!("✅ 测试 1 通过");
    Ok(())
}

/// 测试 2: 带圆角的矩形（有 FRADIUS）
fn test_rectangle_with_fradius() -> Result<()> {
    println!("📋 测试 2: 带圆角的矩形（有 FRADIUS）");
    println!("─────────────────────────────────────");

    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),      // 无圆角
        Vec3::new(100.0, 0.0, 0.0),    // 无圆角
        Vec3::new(100.0, 100.0, 15.0), // 圆角半径 15
        Vec3::new(0.0, 100.0, 10.0),   // 圆角半径 10
    ];

    println!("输入顶点数: {}", vertices.len());
    println!(
        "输入 FRADIUS 顶点数: {}",
        vertices.iter().filter(|v| v.z > 0.0).count()
    );

    // 使用 process_ploop_vertices 处理
    let processed = process_ploop_vertices(&vertices, "RECTANGLE_WITH_FRADIUS")?;

    println!("处理后顶点数: {}", processed.len());

    // 验证结果
    assert!(processed.len() >= 4, "处理后应该至少有 4 个顶点");

    // 打印处理后的顶点
    println!("\n处理后的顶点列表:");
    for (i, v) in processed.iter().enumerate() {
        if v.z.abs() > f32::EPSILON {
            println!("  [{}] ({:.2}, {:.2}) bulge: {:.4}", i, v.x, v.y, v.z);
        } else {
            println!("  [{}] ({:.2}, {:.2})", i, v.x, v.y);
        }
    }

    // 生成 Polyline
    let polyline = gen_polyline(&vertices)?;
    println!("\n生成的 Polyline 顶点数: {}", polyline.vertex_data.len());
    println!("Polyline 是否闭合: {}", polyline.is_closed);

    // 检查圆弧段
    let arc_count = polyline
        .vertex_data
        .iter()
        .filter(|v| v.bulge.abs() > 0.001)
        .count();
    println!("包含圆弧段数: {}", arc_count);

    println!("✅ 测试 2 通过");
    Ok(())
}

/// 测试 3: 复杂形状（多个 FRADIUS）
fn test_complex_shape() -> Result<()> {
    println!("📋 测试 3: 复杂形状（多个 FRADIUS）");
    println!("─────────────────────────────────────");

    let vertices = vec![
        Vec3::new(0.0, 0.0, 5.0),      // 圆角半径 5
        Vec3::new(100.0, 0.0, 8.0),    // 圆角半径 8
        Vec3::new(150.0, 50.0, 0.0),   // 无圆角
        Vec3::new(100.0, 100.0, 12.0), // 圆角半径 12
        Vec3::new(0.0, 100.0, 10.0),   // 圆角半径 10
        Vec3::new(-20.0, 50.0, 0.0),   // 无圆角
    ];

    println!("输入顶点数: {}", vertices.len());
    println!(
        "输入 FRADIUS 顶点数: {}",
        vertices.iter().filter(|v| v.z > 0.0).count()
    );

    // 使用 process_ploop_vertices 处理
    let processed = process_ploop_vertices(&vertices, "COMPLEX_SHAPE")?;

    println!("处理后顶点数: {}", processed.len());

    // 验证结果
    assert!(processed.len() >= 6, "处理后应该至少有 6 个顶点");

    // 打印处理后的顶点
    println!("\n处理后的顶点列表:");
    for (i, v) in processed.iter().enumerate() {
        if v.z.abs() > f32::EPSILON {
            println!("  [{}] ({:.2}, {:.2}) bulge: {:.4}", i, v.x, v.y, v.z);
        } else {
            println!("  [{}] ({:.2}, {:.2})", i, v.x, v.y);
        }
    }

    // 生成 Polyline
    let polyline = gen_polyline(&vertices)?;
    println!("\n生成的 Polyline 顶点数: {}", polyline.vertex_data.len());
    println!("Polyline 是否闭合: {}", polyline.is_closed);

    // 检查圆弧段
    let arc_count = polyline
        .vertex_data
        .iter()
        .filter(|v| v.bulge.abs() > 0.001)
        .count();
    println!("包含圆弧段数: {}", arc_count);

    // 检查是否有 NaN
    let has_nan = polyline.vertex_data.iter().any(|v| v.bulge.is_nan());
    assert!(!has_nan, "不应该有 NaN bulge 值");

    println!("✅ 测试 3 通过");
    Ok(())
}
