//! 测试特定 PLOOP 的 SVG 生成
//!
//! 测试指定参考号的 PLOOP 生成
//!
//! 运行方法：
//! ```bash
//! cargo run --example test_specific_ploop
//! ```

use aios_core::types::RefU64;
use aios_core::{SUL_DB, SurrealQueryExt, init_test_surreal};
use anyhow::Result;
use glam::Vec3;
use ploop_rs::{PloopProcessor, SvgGenerator, Vertex};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 测试特定 PLOOP 的 SVG 生成");
    println!("═══════════════════════════════════════════════════\n");

    // 初始化数据库
    println!("📊 初始化数据库连接...");
    init_test_surreal().await;
    println!("✅ 数据库连接成功\n");

    // 指定要测试的参考号
    let ploop_id: RefU64 = "21909_41078".into();
    println!("🎯 测试 PLOOP: {}\n", ploop_id);

    // 创建输出目录
    let output_dir = Path::new("test_output/specific_ploop");
    fs::create_dir_all(output_dir)?;
    println!("📁 输出目录: {}\n", output_dir.display());

    // 处理 PLOOP
    match process_ploop(ploop_id, output_dir).await {
        Ok(info) => {
            println!("\n✅ 处理成功！");
            println!("📊 {}", info);
        }
        Err(e) => {
            println!("\n❌ 处理失败: {}", e);
            return Err(e);
        }
    }

    println!("\n✅ 测试完成！");
    println!("📁 SVG 文件保存在: {}", output_dir.display());

    Ok(())
}

/// 处理单个 PLOOP
async fn process_ploop(ploop_id: RefU64, output_dir: &Path) -> Result<String> {
    println!("─────────────────────────────────────────────────");
    println!("🔍 查询 PLOOP 数据...");

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

    println!("✅ 查询到 {} 个顶点", raw_vertices.len());

    // 转换为 Vec3 格式：x, y 为坐标，z 存储 FRADIUS 值
    let vertices: Vec<Vec3> = raw_vertices
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let x = v.get(0).copied().unwrap_or_default() as f32;
            let y = v.get(1).copied().unwrap_or_default() as f32;
            let fradius = v.get(2).copied().unwrap_or_default() as f32;
            let vert = Vec3::new(x, y, fradius);
            println!(
                "  顶点 {}: x={:.2}, y={:.2}, fradius={:.2}",
                i + 1,
                x,
                y,
                fradius
            );
            vert
        })
        .collect();

    if vertices.len() < 3 {
        return Err(anyhow::anyhow!(
            "PLOOP {} 顶点数不足: {}",
            ploop_id,
            vertices.len()
        ));
    }

    println!("\n📐 顶点统计:");
    println!("  总顶点数: {}", vertices.len());

    // 统计 FRADIUS 顶点
    let fradius_count = vertices.iter().filter(|v| v.z > 0.0).count();
    println!("  FRADIUS 顶点数: {}", fradius_count);

    // 打印 FRADIUS 顶点详情
    if fradius_count > 0 {
        println!("\n🔵 FRADIUS 顶点详情:");
        for (i, v) in vertices.iter().enumerate() {
            if v.z > 0.0 {
                println!(
                    "    顶点 {}: ({:.2}, {:.2}) FRADIUS={:.2}",
                    i + 1,
                    v.x,
                    v.y,
                    v.z
                );
            }
        }
    }

    // 转换为 ploop-rs 的 Vertex 格式
    let ploop_vertices: Vec<Vertex> = vertices
        .iter()
        .map(|v| {
            if v.z > 0.0 {
                Vertex::with_fradius(v.x, v.y, 0.0, Some(v.z))
            } else {
                Vertex::new(v.x, v.y)
            }
        })
        .collect();

    // 使用 ploop-rs 处理顶点
    println!("\n🔧 使用 ploop-rs 处理顶点...");
    let processor = PloopProcessor::new(0.01, true);
    let (processed_vertices, _bulges, arcs, _reports) = processor.process_ploop(&ploop_vertices);

    println!("  处理后顶点数: {}", processed_vertices.len());
    println!("  生成圆弧数: {}", arcs.len());

    // 打印圆弧详情
    if !arcs.is_empty() {
        println!("\n🌀 圆弧详情:");
        for (i, arc) in arcs.iter().enumerate() {
            println!(
                "    圆弧 {}: 半径={:.1}mm, 扫掠角={:.1}°, 方向={}",
                i,
                arc.radius,
                arc.sweep_degrees(),
                arc.direction_str()
            );
        }
    }

    // 生成 SVG（使用 ploop-rs 的 SvgGenerator）
    println!("\n🎨 生成 SVG...");
    let svg_path = output_dir.join(format!("ploop_{}.svg", ploop_id.0));
    let svg_gen = SvgGenerator::new(1500.0, 50.0);
    svg_gen.generate(&processed_vertices, &arcs, Some(&ploop_vertices), &svg_path)?;
    println!("  SVG 文件: {}", svg_path.display());

    Ok(format!(
        "顶点:{}, FRADIUS:{}, 圆弧:{}, SVG:{}",
        vertices.len(),
        fradius_count,
        arcs.len(),
        svg_path.file_name().unwrap().to_string_lossy()
    ))
}
