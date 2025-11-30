//! 查询 Noun 属性示例
//!
//! 用法: cargo run --example query_noun_attrs [NOUN_NAME] [ATTLIB_PATH]
//!
//! 示例:
//!   cargo run --example query_noun_attrs ELBO
//!   cargo run --example query_noun_attrs EQUI
//!   cargo run --example query_noun_attrs CYLI

use aios_core::noun_attributes::{db1_dehash, db1_hash, AttributeDesc, NounAttributeStore};
use std::env;
use std::path::Path;

fn main() {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();
    let noun_name = args.get(1).map(|s| s.as_str()).unwrap_or("ELBO");
    let attlib_path = args.get(2).map(|s| s.as_str());

    println!("=== PDMS Noun 属性查询 ===\n");

    // 优先从 all_attr_info.json 加载（包含所有 noun）
    let all_attr_path = concat!(env!("CARGO_MANIFEST_DIR"), "/all_attr_info.json");
    
    let store = if std::path::Path::new(all_attr_path).exists() {
        println!("📂 数据文件: {}\n", all_attr_path);
        match NounAttributeStore::load_from_all_attr_info(all_attr_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("❌ 加载 all_attr_info.json 失败: {}", e);
                return;
            }
        }
    } else {
        // 回退到目录加载
        let data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        println!("📂 数据目录: {}\n", data_dir);
        match NounAttributeStore::load_from_directory(data_dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("❌ 加载失败: {}", e);
                return;
            }
        }
    };

    // 显示已加载的 noun 列表
    let mut loaded_nouns = store.get_loaded_nouns();
    loaded_nouns.sort();
    println!("✅ 已加载 {} 个 noun 类型", loaded_nouns.len());
    
    // 只显示部分 noun
    let display_nouns: Vec<_> = loaded_nouns.iter().take(15).collect();
    for noun in &display_nouns {
        let count = store.get_attribute_count(noun);
        println!("   - {} ({} 属性)", noun, count);
    }
    if loaded_nouns.len() > 15 {
        println!("   ... 还有 {} 个", loaded_nouns.len() - 15);
    }
    println!();

    // 查询指定 noun
    let noun_upper = noun_name.to_uppercase();
    let noun_hash = db1_hash(&noun_upper);
    if let Some(path) = attlib_path {
        println!("🔗 attlib.dat: {}\n", path);
    }
    println!("🔍 查询: {} (hash=0x{:08X})", noun_upper, noun_hash);
    println!();

    let descs: Vec<AttributeDesc> = match store.describe_noun(&noun_upper, attlib_path.map(Path::new)) {
        Ok(list) => list,
        Err(e) => {
            eprintln!("❌ 获取 {} 属性失败: {}", noun_upper, e);
            println!("\n💡 提示: 确认 all_attr_info.json 或 data/ 下存在对应 noun 数据");
            return;
        }
    };

    println!("📋 {} 的属性列表 ({} 个):\n", noun_upper, descs.len());

    // 按类型分组显示
    let mut by_type: std::collections::HashMap<String, Vec<&AttributeDesc>> =
        std::collections::HashMap::new();
    for desc in &descs {
        by_type
            .entry(desc.att_type.to_string())
            .or_default()
            .push(desc);
    }

    for (type_name, type_attrs) in by_type.iter() {
        println!("  【{}】({} 个)", type_name, type_attrs.len());
        for desc in type_attrs {
            let attlib_hint = if let Some(t) = &desc.attlib_type {
                if let Some(def) = &desc.attlib_default {
                    format!(" attlib=({}: {})", t, def)
                } else {
                    format!(" attlib=({})", t)
                }
            } else {
                String::new()
            };
            println!(
                "    {:12} hash=0x{:08X} offset={} default={}{}",
                desc.name, desc.hash, desc.offset, desc.default_val, attlib_hint
            );
        }
        println!();
    }

    // 显示特定属性详情
    println!("📝 属性详情示例:");
    for attr_name in ["NAME", "POS", "ORI", "TYPE"].iter() {
        if let Some(attr) = descs.iter().find(|a| a.name.eq_ignore_ascii_case(attr_name)) {
            println!("  {}:", attr.name);
            println!("    - Hash: 0x{:08X}", attr.hash);
            println!("    - Type: {}", attr.att_type);
            println!("    - Offset: {}", attr.offset);
            println!("    - Default: {}", attr.default_val);
            if let Some(t) = &attr.attlib_type {
                println!("    - attlib 类型: {}", t);
            }
            if let Some(d) = &attr.attlib_default {
                println!("    - attlib 默认值: {}", d);
            }
        }
    }

    // Hash 转换示例
    println!("\n=== Hash 转换工具 ===\n");
    let test_names = ["ELBO", "PIPE", "NAME", "POS", "ORI"];
    for name in test_names {
        let hash = db1_hash(name);
        let decoded = db1_dehash(hash).unwrap_or_default();
        println!("  {} → 0x{:08X} → {}", name, hash, decoded);
    }
}
