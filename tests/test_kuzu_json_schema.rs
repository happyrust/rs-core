//! 测试 Kuzu JSON schema 生成功能

#[cfg(feature = "kuzu")]
mod tests {
    use aios_core::rs_kuzu::json_schema::{
        generate_all_table_sqls, generate_noun_table_sql, load_attr_info_json,
        pdms_type_to_kuzu,
    };
    use aios_core::pdms_types::DbAttributeType;
    use aios_core::types::AttrVal;

    #[test]
    fn test_load_json_and_generate_schema() {
        // 测试加载 JSON
        let attr_info = load_attr_info_json().expect("应该能加载 all_attr_info.json");

        println!("成功加载 {} 个 noun", attr_info.named_attr_info_map.len());

        // 测试为 ELBO 生成表
        if let Some(elbo_attrs) = attr_info.named_attr_info_map.get("ELBO") {
            let sql = generate_noun_table_sql("ELBO", elbo_attrs)
                .expect("应该能生成 ELBO 表 SQL");

            println!("\n生成的 ELBO 表 SQL:");
            println!("{}", sql);

            // 验证 SQL 包含预期内容
            assert!(sql.contains("CREATE NODE TABLE IF NOT EXISTS Attr_ELBO"));
            assert!(sql.contains("refno INT64 PRIMARY KEY"));
            assert!(sql.contains("STATUS_CODE"));
            assert!(sql.contains("SPRE_REFNO"));
        }

        // 测试为 PIPE 生成表
        if let Some(pipe_attrs) = attr_info.named_attr_info_map.get("PIPE") {
            let sql = generate_noun_table_sql("PIPE", pipe_attrs)
                .expect("应该能生成 PIPE 表 SQL");

            println!("\n生成的 PIPE 表 SQL:");
            println!("{}", sql);

            assert!(sql.contains("CREATE NODE TABLE IF NOT EXISTS Attr_PIPE"));
        }
    }

    #[test]
    fn test_generate_all_sqls() {
        let sqls = generate_all_table_sqls().expect("应该能生成所有表的 SQL");

        println!("\n总共生成 {} 条 SQL 语句", sqls.len());

        // 检查是否包含 PE 主表
        let has_pe = sqls.iter().any(|sql| sql.contains("CREATE NODE TABLE IF NOT EXISTS PE"));
        assert!(has_pe, "应该包含 PE 主表");

        // 检查是否包含 OWNS 关系表
        let has_owns = sqls.iter().any(|sql| sql.contains("CREATE REL TABLE IF NOT EXISTS OWNS"));
        assert!(has_owns, "应该包含 OWNS 关系表");

        // 检查是否包含属性表
        let attr_tables = sqls.iter().filter(|sql| sql.contains("Attr_")).count();
        println!("生成了 {} 个属性表", attr_tables);
        assert!(attr_tables > 0, "应该至少有一个属性表");

        // 检查是否包含 TO_ 关系表
        let to_relations = sqls.iter().filter(|sql| sql.contains("TO_")).count();
        println!("生成了 {} 个 TO_ 关系表", to_relations);
        assert!(to_relations > 0, "应该至少有一个 TO_ 关系表");

        // 打印前几个 SQL 作为示例
        println!("\n前 5 个 SQL 语句:");
        for (i, sql) in sqls.iter().take(5).enumerate() {
            println!("\n[{}] {}", i + 1, sql);
        }
    }

    #[test]
    fn test_type_mapping() {
        // 测试基本类型映射
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::INTEGER, &AttrVal::IntegerType(0)),
            "INT32"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::DOUBLE, &AttrVal::DoubleType(0.0)),
            "DOUBLE"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::STRING, &AttrVal::StringType("".into())),
            "STRING"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::BOOL, &AttrVal::BoolType(false)),
            "BOOLEAN"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::ELEMENT, &AttrVal::RefU64Type(0.into())),
            "INT64"
        );

        // 测试数组类型
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::ORIENTATION, &AttrVal::Vec3Type([0.0, 0.0, 0.0])),
            "LIST<DOUBLE>"
        );

        println!("类型映射测试通过");
    }

    #[test]
    fn test_specific_noun_attrs() {
        let attr_info = load_attr_info_json().expect("应该能加载 JSON");

        // 检查一些重要的 noun
        let important_nouns = ["ELBO", "PIPE", "EQUIPMENT", "SITE", "ZONE"];

        for noun in &important_nouns {
            if let Some(attrs) = attr_info.named_attr_info_map.get(*noun) {
                println!("\n{} 有 {} 个属性", noun, attrs.len());

                // 打印前 10 个属性
                let mut attr_names: Vec<_> = attrs.keys().collect();
                attr_names.sort();

                println!("{} 的前 10 个属性:", noun);
                for (i, name) in attr_names.iter().take(10).enumerate() {
                    if let Some(info) = attrs.get(*name) {
                        println!("  {}. {} (类型: {:?})", i + 1, name, info.att_type);
                    }
                }
            } else {
                println!("{} noun 不存在", noun);
            }
        }
    }
}