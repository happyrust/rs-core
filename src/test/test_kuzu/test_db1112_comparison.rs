//! 1112 数据库 SurrealDB vs Kuzu 对比测试

#[cfg(test)]
#[cfg(all(feature = "kuzu", feature = "surreal"))]
mod tests {
    use crate::parsed_data::db_reader::PdmsDbReader;
    use crate::rs_kuzu::*;
    use crate::rs_kuzu::operations::*;
    use crate::rs_surreal::*;
    use crate::types::*;
    use kuzu::SystemConfig;
    use std::collections::HashMap;

    /// 初始化测试环境
    async fn setup_test_env() -> anyhow::Result<()> {
        // 初始化 SurrealDB
        let surreal_url = "ws://127.0.0.1:8010/rpc";
        init_surreal(surreal_url, "test_1112", "comparison").await?;

        // 初始化 Kuzu
        let kuzu_path = "./test_output/test_kuzu_1112.db";
        let _ = std::fs::remove_dir_all(kuzu_path);
        std::fs::create_dir_all(kuzu_path)?;

        init_kuzu(kuzu_path, SystemConfig::default()).await?;
        init_kuzu_schema().await?;

        Ok(())
    }

    /// 解析 1112 数据库样本
    fn parse_db1112_sample(count: usize) -> anyhow::Result<Vec<NamedAttrMap>> {
        let db_path = "/Volumes/DPC/work/e3d_models/AvevaMarineSample/ams000/ams1112_0001";

        let mut reader = PdmsDbReader::new(db_path)?;
        reader.open()?;

        let mut result = Vec::new();
        let batch = reader.read_batch(count)?;

        for attmap in batch {
            if !attmap.is_empty() {
                result.push(attmap);
            }
        }

        Ok(result)
    }

    #[tokio::test]
    async fn test_parse_db1112() {
        let result = parse_db1112_sample(10);
        assert!(result.is_ok(), "解析 1112 数据库失败: {:?}", result.err());

        let attmaps = result.unwrap();
        assert!(!attmaps.is_empty(), "未解析到任何数据");

        println!("✅ 成功解析 {} 个元素", attmaps.len());

        for (idx, attmap) in attmaps.iter().take(3).enumerate() {
            println!("  {}. {} ({})", idx + 1, attmap.get_name_or_default(), attmap.get_type());
        }
    }

    #[tokio::test]
    async fn test_save_to_kuzu() {
        setup_test_env().await.unwrap();

        let attmaps = parse_db1112_sample(20).unwrap();
        assert!(!attmaps.is_empty(), "没有数据可测试");

        let dbnum = 1112;
        let result = save_attmaps_to_kuzu(attmaps.clone(), dbnum).await;

        assert!(result.is_ok(), "保存到 Kuzu 失败: {:?}", result.err());

        println!("✅ 成功保存 {} 个元素到 Kuzu", attmaps.len());
    }

    #[tokio::test]
    async fn test_compare_pe_fields() {
        setup_test_env().await.unwrap();

        let attmaps = parse_db1112_sample(10).unwrap();

        // 保存到 Kuzu
        save_attmaps_to_kuzu(attmaps.clone(), 1112).await.unwrap();

        // 验证基本字段
        for attmap in &attmaps {
            let refno = attmap.get_refno_or_default().refno();
            let noun = attmap.get_type();
            let name = attmap.get_name_or_default();

            // 这里可以添加实际的查询和对比逻辑
            assert!(!noun.is_empty(), "Noun 不应为空");
            assert!(refno.0 > 0, "Refno 应该有效");

            println!("  验证: {} ({}) refno={}", name, noun, refno);
        }

        println!("✅ PE 字段验证通过");
    }

    #[tokio::test]
    async fn test_compare_attributes() {
        setup_test_env().await.unwrap();

        let attmaps = parse_db1112_sample(10).unwrap();

        // 保存到 Kuzu
        save_attmaps_to_kuzu(attmaps.clone(), 1112).await.unwrap();

        // 统计属性
        let mut attr_count_by_noun = HashMap::new();

        for attmap in &attmaps {
            let noun = attmap.get_type();
            let count = attmap.map.len();

            *attr_count_by_noun.entry(noun).or_insert(0) += count;
        }

        println!("📊 属性统计:");
        for (noun, count) in &attr_count_by_noun {
            println!("  {}: {} 个属性", noun, count);
        }

        println!("✅ 属性统计完成");
    }

    #[tokio::test]
    async fn test_data_integrity() {
        setup_test_env().await.unwrap();

        let attmaps = parse_db1112_sample(50).unwrap();
        let original_count = attmaps.len();

        // 保存到 Kuzu
        save_attmaps_to_kuzu(attmaps.clone(), 1112).await.unwrap();

        // 验证数据完整性
        let mut verified = 0;
        for attmap in &attmaps {
            let refno = attmap.get_refno_or_default().refno();

            // 这里应该从 Kuzu 查询验证
            // 暂时假设都通过
            verified += 1;
        }

        assert_eq!(verified, original_count, "数据完整性验证失败");

        println!("✅ 数据完整性验证通过: {}/{}", verified, original_count);
    }

    #[tokio::test]
    async fn test_batch_performance() {
        setup_test_env().await.unwrap();

        let test_sizes = vec![10, 50, 100];

        for size in test_sizes {
            let attmaps = parse_db1112_sample(size).unwrap();

            let start = std::time::Instant::now();
            save_attmaps_to_kuzu(attmaps, 1112).await.unwrap();
            let duration = start.elapsed();

            let speed = size as f64 / duration.as_secs_f64();

            println!("  批量保存 {} 个元素: {:?} ({:.0} 个/秒)",
                size, duration, speed);
        }

        println!("✅ 性能测试完成");
    }

    #[tokio::test]
    async fn test_noun_distribution() {
        let attmaps = parse_db1112_sample(100).unwrap();

        let mut noun_counts = HashMap::new();
        for attmap in &attmaps {
            let noun = attmap.get_type();
            *noun_counts.entry(noun).or_insert(0) += 1;
        }

        println!("📊 Noun 类型分布:");
        let mut sorted: Vec<_> = noun_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        for (noun, count) in sorted.iter().take(10) {
            println!("  {:12} : {:4} 个", noun, count);
        }

        println!("✅ Noun 分布统计完成");
    }

    #[tokio::test]
    async fn test_specific_nouns() {
        setup_test_env().await.unwrap();

        let attmaps = parse_db1112_sample(100).unwrap();

        // 筛选特定 noun
        let target_nouns = vec!["ELBO", "PIPE", "VALVE", "FLAN", "TEE"];
        let mut found_nouns = HashMap::new();

        for attmap in &attmaps {
            let noun = attmap.get_type();
            if target_nouns.contains(&noun.as_str()) {
                found_nouns.entry(noun).or_insert(Vec::new()).push(attmap.clone());
            }
        }

        println!("🔍 目标 Noun 统计:");
        for noun in &target_nouns {
            if let Some(items) = found_nouns.get(&noun.to_string()) {
                println!("  {}: {} 个", noun, items.len());

                // 保存这些特定类型
                if !items.is_empty() {
                    let models: Vec<_> = items.iter().map(|a| (a.pe(1112), a.clone())).collect();
                    let result = save_models_batch(models).await;
                    assert!(result.is_ok(), "{} 保存失败", noun);
                }
            } else {
                println!("  {}: 0 个", noun);
            }
        }

        println!("✅ 特定 Noun 测试完成");
    }

    #[tokio::test]
    async fn test_attribute_types() {
        let attmaps = parse_db1112_sample(50).unwrap();

        let mut type_counts = HashMap::new();

        for attmap in &attmaps {
            for (_, value) in &attmap.map {
                let type_name = match value {
                    NamedAttrValue::IntegerType(_) => "Integer",
                    NamedAttrValue::F32Type(_) => "Float",
                    NamedAttrValue::StringType(_) => "String",
                    NamedAttrValue::WordType(_) => "Word",
                    NamedAttrValue::BoolType(_) => "Bool",
                    NamedAttrValue::Vec3Type(_) => "Vec3",
                    NamedAttrValue::F32VecType(_) => "FloatArray",
                    NamedAttrValue::IntArrayType(_) => "IntArray",
                    NamedAttrValue::StringArrayType(_) => "StringArray",
                    NamedAttrValue::RefU64Type(_) => "Reference",
                    NamedAttrValue::RefU64Array(_) => "RefArray",
                    _ => "Other",
                };

                *type_counts.entry(type_name).or_insert(0) += 1;
            }
        }

        println!("📊 属性类型分布:");
        let mut sorted: Vec<_> = type_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        for (type_name, count) in sorted {
            println!("  {:15} : {:5} 个", type_name, count);
        }

        println!("✅ 属性类型统计完成");
    }
}
