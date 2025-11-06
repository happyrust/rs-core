#!/usr/bin/env rust-script
//! 测试 FLOOR PLOOP 数据
//! 
//! 使用方法：
//! ```bash
//! cd /Volumes/DPC/work/plant-code/rust-ploop-processor/rust-ploop-processor
//! cargo run --bin test_floor_ploop
//! ```

use rust_ploop_processor::*;
use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    println!("🧪 测试 FLOOR PLOOP 数据");
    println!("═══════════════════════════════════\n");

    // 读取 TXT 文件
    let txt_path = "/Volumes/DPC/work/plant-code/gen-model/output/ploop-json/ploop_FLOOR_1762197834.txt";
    let content = fs::read_to_string(txt_path)?;
    
    println!("📄 读取文件: {}", txt_path);
    println!("文件内容:\n{}\n", content);

    // 解析 PLOOP
    let parser = PLoopParser::new(1.0);
    let ploops = parser.parse_file(&content)?;
    
    println!("✅ 解析成功! 发现 {} 个PLOOP\n", ploops.len());
    
    if let Some(ploop) = ploops.first() {
        println!("📊 PLOOP 信息:");
        println!("   名称: {}", ploop.name);
        println!("   高度: {:.1}mm", ploop.height);
        println!("   原始顶点数: {}", ploop.vertices.len());
        
        // 统计 FRADIUS 顶点
        let fradius_count = ploop.vertices.iter().filter(|v| v.has_fradius()).count();
        println!("   FRADIUS 顶点数: {}", fradius_count);
        
        println!("\n📍 原始顶点列表:");
        for (i, v) in ploop.vertices.iter().enumerate() {
            if v.has_fradius() {
                println!("  [{}] ({:.1}, {:.1}) FRADIUS: {:.1}mm 🔵",
                    i, v.x(), v.y(), v.get_fradius());
            } else {
                println!("  [{}] ({:.1}, {:.1})",
                    i, v.x(), v.y());
            }
        }
        
        // 处理 PLOOP
        println!("\n🔧 开始处理 PLOOP...");
        let processor = PLoopProcessor::new();
        let processed = processor.process_ploop(ploop)?;
        
        println!("✅ 处理完成!");
        println!("   处理后顶点数: {}", processed.len());
        
        println!("\n📍 处理后顶点列表:");
        for (i, v) in processed.iter().enumerate() {
            if v.has_fradius() {
                println!("  [{}] ({:.1}, {:.1}) FRADIUS: {:.1}mm ⚠️",
                    i, v.x(), v.y(), v.get_fradius());
            } else {
                println!("  [{}] ({:.1}, {:.1})",
                    i, v.x(), v.y());
            }
        }
        
        // 导出 JSON
        let json_output = "/Volumes/DPC/work/plant-code/gen-model/output/ploop-json/processed_floor.json";
        JsonExporter::export_ploop(ploop, &processed, json_output)?;
        println!("\n💾 处理结果已保存到: {}", json_output);
        
        // 生成 SVG
        let svg_output = "/Volumes/DPC/work/plant-code/gen-model/output/ploop-json/floor_ploop.svg";
        let svg_gen = SimpleSvgGenerator::new();
        svg_gen.generate_svg(ploop, &processed, svg_output)?;
        println!("🎨 SVG 已保存到: {}", svg_output);
    }
    
    println!("\n✅ 测试完成!");
    
    Ok(())
}

