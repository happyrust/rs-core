use aios_core::attlib_parser::AttlibParser;
use serde_json::{json, Value};
use std::collections::HashMap;

// ELBO 属性的哈希值映射（从 data/ELBO.json 中提取）
fn get_elbo_attributes() -> HashMap<&'static str, u32> {
    let mut map = HashMap::new();
    map.insert("RLIN", 813906);
    map.insert("ISPE", 642042);
    map.insert("TSPE", 642053);
    map.insert("DPGRID", 63069871);
    map.insert("OWNER", 10206636);
    map.insert("LEXCES", 275894571);
    map.insert("ORIL", 774699);
    map.insert("DELDSG", 111158788);
    map.insert("RLOC", 601767);
    map.insert("RADI", 711549);
    map.insert("INPRTR", 269806932);
    map.insert("ANGL", 773119);
    map.insert("JNUM", 803017);
    map.insert("ORRF", 663162);
    map.insert("DESP", 860359);
    map.insert("SPRE", 643429);
    map.insert("SHOP", 857539);
    map.insert("HREL", 771776);
    map.insert("CREF", 653673);
    map.insert("OUPRTR", 269807127);
    map.insert("LOCK", 750558);
    map.insert("MTOC", 601978);
    map.insert("DPFN", 811813);
    map.insert("PTNB", 581569);
    map.insert("SPLN", 816202);
    map.insert("POSI", 722860);
    map.insert("DMTY", 1038451);
    map.insert("BSEL", 771797);
    map.insert("MTOR", 897223);
    map.insert("PTNO", 837448);
    map.insert("ORI", 538503);
    map.insert("NWELDS", 275526914);
    map.insert("TSFBR", 10141652);
    map.insert("BUIL", 774767);
    map.insert("ORIF", 656601);
    map.insert("LOOS", 916770);
    map.insert("JOIP", 853345);
    map.insert("ISOH", 700362);
    map.insert("TYPE", 642215);
    map.insert("LSTU", 959889);
    map.insert("POS", 545713);
    map.insert("PTNT", 935863);
    map.insert("NAME", 639374);
    map.insert("INVF", 665964);
    map.insert("RLEX", 1007820);
    map.insert("AEXCES", 275894560);
    map.insert("SPLT", 934300);
    map.insert("MTOT", 936589);
    map.insert("DMFA", 555853);
    map.insert("CSFBR", 10141635);
    map.insert("ARRI", 722197);
    map.insert("CMPX", 1015851);
    map.insert("MTOX", 1015321);
    map.insert("SPSP", 860671);
    map.insert("LEAV", 965343);
    map
}

fn data_type_to_string(data_type: u32) -> String {
    match data_type {
        1 => "LOG".to_string(),
        2 => "REAL".to_string(),
        3 => "INT".to_string(),
        4 => "TEXT".to_string(),
        _ => format!("UNKNOWN({})", data_type),
    }
}

fn main() -> std::io::Result<()> {
    println!("=== ELBO 属性解析 ===\n");

    let mut parser = AttlibParser::new("data/attlib.dat")?;
    parser.load_all()?;

    let elbo_attrs = get_elbo_attributes();
    let mut result = json!({
        "ELBO": {}
    });

    let mut found_count = 0;
    let mut missing_count = 0;

    for (name, hash) in &elbo_attrs {
        match parser.get_attribute(*hash) {
            Some(attr_def) => {
                found_count += 1;
                let default_val = match &attr_def.default_value {
                    aios_core::attlib_parser::AttlibDefaultValue::None => json!(null),
                    aios_core::attlib_parser::AttlibDefaultValue::Scalar(v) => json!(v),
                    aios_core::attlib_parser::AttlibDefaultValue::Text(v) => json!(v),
                };

                result["ELBO"][name] = json!({
                    "name": name,
                    "hash": hash,
                    "data_type": data_type_to_string(attr_def.data_type),
                    "data_type_code": attr_def.data_type,
                    "default_flag": attr_def.default_flag,
                    "default_value": default_val,
                });

                println!("✓ {} (hash: {}, type: {})", name, hash, data_type_to_string(attr_def.data_type));
            }
            None => {
                missing_count += 1;
                println!("✗ {} (hash: {}) - NOT FOUND", name, hash);
            }
        }
    }

    println!("\n=== 统计信息 ===");
    println!("总属性数: {}", elbo_attrs.len());
    println!("找到: {}", found_count);
    println!("缺失: {}", missing_count);

    // 输出 JSON 结果
    println!("\n=== JSON 输出 ===");
    println!("{}", serde_json::to_string_pretty(&result)?);

    // 保存到文件
    let output_path = "test_output/elbo_parsed.json";
    std::fs::create_dir_all("test_output")?;
    std::fs::write(output_path, serde_json::to_string_pretty(&result)?)?;
    println!("\n✓ 结果已保存到: {}", output_path);

    Ok(())
}

