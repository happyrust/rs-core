//! attlib.dat 解析示例
//!
//! 运行方式:
//! ```bash
//! cargo run --example parse_attlib
//! cargo run --example parse_attlib -- --json    # 导出 JSON
//! cargo run --example parse_attlib -- --syntax  # 加载语法表
//! ```

use aios_core::attlib_parser::{AttlibParser, AttlibDataType, encode_base27};

fn main() -> std::io::Result<()> {
    let attlib_path = "data/attlib.dat";
    
    println!("=== attlib.dat 解析器 ===\n");
    
    // 创建解析器
    let mut parser = AttlibParser::new(attlib_path)?;
    
    // 读取文件头
    if let Ok(header) = parser.read_header() {
        if !header.trim().is_empty() {
            println!("文件头: {}\n", header);
        }
    }
    
    // 加载所有数据
    parser.load_all()?;
    
    // 显示统计信息
    println!("\n=== 统计信息 ===");
    println!("{:#?}", parser.stats);
    
    // 加载语法表（类型-属性映射）
    if std::env::args().any(|arg| arg == "--syntax") {
        println!("\n=== 加载语法表（类型-属性映射） ===");
        
        let noun_map = parser.load_syntax()?;
        
        println!("\n类型数量: {}", noun_map.noun_count());
        
        // 显示前 10 个类型及其属性
        println!("\n前 10 个类型:");
        for (i, noun_name) in noun_map.all_nouns().iter().take(10).enumerate() {
            let noun_hash = encode_base27(noun_name);
            let attrs = noun_map.get_attribute_names(noun_hash);
            println!("  {}. {} ({} 个属性)", i + 1, noun_name, attrs.len());
            for attr in attrs.iter().take(5) {
                println!("      - {}", attr);
            }
            if attrs.len() > 5 {
                println!("      ... 还有 {} 个", attrs.len() - 5);
            }
        }
        
        // 查询特定类型的属性
        let test_nouns = ["EQUI", "PIPE", "NOZZ", "SITE", "ZONE"];
        println!("\n=== 查询特定类型的属性 ===");
        for noun in test_nouns {
            let attrs = noun_map.get_attributes_by_name(noun);
            if !attrs.is_empty() {
                println!("\n{} 的属性 ({} 个):", noun, attrs.len());
                for attr in attrs.iter().take(10) {
                    println!("  - {}", attr);
                }
                if attrs.len() > 10 {
                    println!("  ... 还有 {} 个", attrs.len() - 10);
                }
            } else {
                println!("\n{}: 未找到属性", noun);
            }
        }
    }
    
    // 按类型分组显示属性
    println!("\n=== 属性列表（按类型分组） ===");
    
    let all_attrs = parser.list_all_attributes();
    
    // 按类型分组
    let mut by_type: std::collections::HashMap<AttlibDataType, Vec<_>> = std::collections::HashMap::new();
    for attr in &all_attrs {
        by_type.entry(attr.data_type).or_default().push(attr);
    }
    
    for data_type in [AttlibDataType::Log, AttlibDataType::Int, AttlibDataType::Real, AttlibDataType::Text] {
        if let Some(attrs) = by_type.get(&data_type) {
            println!("\n--- {:?} 类型 ({} 个) ---", data_type, attrs.len());
            for attr in attrs.iter().take(10) {
                let default_str = attr.default_text.as_deref().unwrap_or("-");
                println!("  {:10} hash=0x{:08X}  默认值: {}", 
                    attr.name, attr.hash, default_str);
            }
            if attrs.len() > 10 {
                println!("  ... 还有 {} 个", attrs.len() - 10);
            }
        }
    }
    
    // 导出 JSON（可选）
    if std::env::args().any(|arg| arg == "--json") {
        println!("\n=== 导出 JSON ===");
        if let Ok(json) = parser.export_json() {
            let output_path = "attlib_parsed.json";
            std::fs::write(output_path, &json)?;
            println!("已导出到: {} ({} 字节)", output_path, json.len());
        }
    }
    
    Ok(())
}
