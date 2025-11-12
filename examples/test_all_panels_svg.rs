//! 测试所有 PLOOP 的 SVG 生成
//!
//! 从数据库查询所有 PLOOP，生成 SVG 来验证 wire 的正确性
//!
//! 运行方法：
//! ```bash
//! cargo run --example test_all_panels_svg
//! ```

use aios_core::prim_geo::wire::{gen_polyline, process_ploop_vertices};
use aios_core::types::RefU64;
use aios_core::{SUL_DB, SurrealQueryExt, init_test_surreal};
use anyhow::Result;
use cavalier_contours::polyline::Polyline;
use glam::Vec3;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 测试所有 PLOOP 的 SVG 生成");
    println!("═══════════════════════════════════════════════════\n");

    // 初始化数据库
    println!("📊 初始化数据库连接...");
    init_test_surreal().await?;
    println!("✅ 数据库连接成功\n");

    // 查询所有 PLOOP
    println!("🔍 查询所有 PLOOP...");
    let query = "SELECT value REFNO FROM PLOO LIMIT 10";
    let ploop_ids: Vec<RefU64> = SUL_DB.query_take(query, 0).await?;

    println!(
        "✅ 找到 {} 个 PLOOP（限制前10个用于测试）\n",
        ploop_ids.len()
    );

    // 打印前几个 ID
    for (i, id) in ploop_ids.iter().take(5).enumerate() {
        println!(
            "  [{}] PLOOP ID: {} (table_key: {})",
            i + 1,
            id,
            id.to_table_key("PLOO")
        );
    }

    // 创建输出目录
    let output_dir = Path::new("test_output/ploop_svgs");
    fs::create_dir_all(output_dir)?;
    println!("📁 输出目录: {}\n", output_dir.display());

    // 统计信息
    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    // 处理每个 PLOOP
    for (idx, ploop_id) in ploop_ids.iter().enumerate() {
        println!("─────────────────────────────────────────────────");
        println!("处理 PLOOP [{}/{}]: {}", idx + 1, ploop_ids.len(), ploop_id);

        match process_ploop(*ploop_id, output_dir).await {
            Ok(info) => {
                println!("✅ 成功: {}", info);
                success_count += 1;
            }
            Err(e) => {
                println!("❌ 失败: {}", e);
                error_count += 1;
                errors.push((*ploop_id, e.to_string()));
            }
        }
    }

    // 输出统计
    println!("\n═══════════════════════════════════════════════════");
    println!("📊 处理统计:");
    println!("  总数: {}", ploop_ids.len());
    println!(
        "  成功: {} ({:.1}%)",
        success_count,
        success_count as f64 / ploop_ids.len() as f64 * 100.0
    );
    println!(
        "  失败: {} ({:.1}%)",
        error_count,
        error_count as f64 / ploop_ids.len() as f64 * 100.0
    );

    // 输出错误列表
    if !errors.is_empty() {
        println!("\n❌ 错误列表:");
        for (ploop_id, error) in errors.iter() {
            println!("  - PLOOP {}: {}", ploop_id, error);
        }
    }

    println!("\n✅ 测试完成！");
    println!("📁 SVG 文件保存在: {}", output_dir.display());

    Ok(())
}

/// 处理单个 PLOOP
async fn process_ploop(ploop_id: RefU64, output_dir: &Path) -> Result<String> {
    // 获取 PLOOP 的顶点数据
    // 使用 .children.refno 来获取子节点（VERT）的数据
    let query = format!(
        "SELECT value [POS[0], POS[1], FRAD] FROM {}.children.refno",
        ploop_id.to_pe_key()
    );

    println!("  查询语句: {}", query);

    // 查询顶点数据：返回 Vec<Vec<f64>>，每个内部 Vec 包含 [x, y, fradius]
    let raw_vertices: Vec<Vec<f64>> = match SUL_DB.query_take(&query, 0).await {
        Ok(v) => v,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "查询顶点失败: {:?}\n查询语句: {}\nPLOOP ID: {}",
                e,
                query,
                ploop_id
            ));
        }
    };

    if raw_vertices.is_empty() {
        return Err(anyhow::anyhow!("PLOOP {} 没有顶点数据", ploop_id));
    }

    // 转换为 Vec3 格式：x, y 为坐标，z 存储 FRADIUS 值
    let vertices: Vec<Vec3> = raw_vertices
        .into_iter()
        .map(|v| {
            let x = v.get(0).copied().unwrap_or_default() as f32;
            let y = v.get(1).copied().unwrap_or_default() as f32;
            let fradius = v.get(2).copied().unwrap_or_default() as f32;
            Vec3::new(x, y, fradius)
        })
        .collect();

    if vertices.len() < 3 {
        return Err(anyhow::anyhow!(
            "PLOOP {} 顶点数不足: {}",
            ploop_id,
            vertices.len()
        ));
    }

    println!("  顶点数: {}", vertices.len());

    // 统计 FRADIUS 顶点
    let fradius_count = vertices.iter().filter(|v| v.z > 0.0).count();
    println!("  FRADIUS 顶点数: {}", fradius_count);

    // 使用 ploop-rs 处理顶点
    let processed = process_ploop_vertices(&vertices, &format!("PLOOP_{}", ploop_id.0))?;
    println!("  处理后顶点数: {}", processed.len());

    // 生成 Polyline
    let polyline = gen_polyline(&vertices)?;
    println!("  Polyline 顶点数: {}", polyline.vertex_data.len());
    println!("  Polyline 闭合: {}", polyline.is_closed);

    // 检查圆弧段
    let arc_count = polyline
        .vertex_data
        .iter()
        .filter(|v| v.bulge.abs() > 0.001)
        .count();
    println!("  圆弧段数: {}", arc_count);

    // 生成 SVG
    let svg_path = output_dir.join(format!("ploop_{}.svg", ploop_id.0));
    generate_svg(&polyline, &vertices, &svg_path)?;

    Ok(format!(
        "顶点:{}, FRADIUS:{}, 圆弧:{}, SVG:{}",
        vertices.len(),
        fradius_count,
        arc_count,
        svg_path.file_name().unwrap().to_string_lossy()
    ))
}

/// 生成 SVG 文件
fn generate_svg(
    polyline: &Polyline<f64>,
    original_vertices: &[Vec3],
    output_path: &Path,
) -> Result<()> {
    // 计算边界框
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for v in &polyline.vertex_data {
        min_x = min_x.min(v.x);
        max_x = max_x.max(v.x);
        min_y = min_y.min(v.y);
        max_y = max_y.max(v.y);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let padding = 50.0;

    let svg_width = width + 2.0 * padding;
    let svg_height = height + 2.0 * padding;

    // 创建 SVG 文件
    let mut file = File::create(output_path)?;

    // SVG 头部
    writeln!(file, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(
        file,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">"#,
        svg_width,
        svg_height,
        min_x - padding,
        min_y - padding,
        svg_width,
        svg_height
    )?;

    // 绘制原始顶点（红色小圆点）
    for v in original_vertices {
        writeln!(
            file,
            r#"  <circle cx="{}" cy="{}" r="2" fill="red" opacity="0.5"/>"#,
            v.x, v.y
        )?;
    }

    // 绘制 Polyline 路径
    write!(file, r#"  <path d="M"#)?;

    for (i, v) in polyline.vertex_data.iter().enumerate() {
        if i == 0 {
            write!(file, " {},{}", v.x, v.y)?;
        } else {
            let prev = &polyline.vertex_data[i - 1];
            if v.bulge.abs() < 0.001 {
                // 直线段
                write!(file, " L {},{}", v.x, v.y)?;
            } else {
                // 圆弧段
                let radius = ((v.x - prev.x).powi(2) + (v.y - prev.y).powi(2)).sqrt()
                    / (2.0 * v.bulge.abs());
                let large_arc = if v.bulge.abs() > 1.0 { 1 } else { 0 };
                let sweep = if v.bulge > 0.0 { 1 } else { 0 };
                write!(
                    file,
                    " A {},{} 0 {} {} {},{}",
                    radius, radius, large_arc, sweep, v.x, v.y
                )?;
            }
        }
    }

    if polyline.is_closed {
        write!(file, " Z")?;
    }

    writeln!(file, r#"" fill="none" stroke="blue" stroke-width="1"/>"#)?;

    // SVG 尾部
    writeln!(file, "</svg>")?;

    Ok(())
}
