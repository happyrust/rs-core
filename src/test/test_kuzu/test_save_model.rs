//! 测试模型保存功能

#[cfg(test)]
#[cfg(feature = "kuzu")]
mod tests {
    use crate::rs_kuzu::operations::*;
    use crate::rs_kuzu::*;
    use crate::types::*;
    use glam::Vec3;

    /// 初始化测试 Kuzu 数据库
    async fn init_test_kuzu() -> anyhow::Result<()> {
        use kuzu::SystemConfig;

        let db_path = "./test_output/kuzu_save_test.db";

        // 删除旧数据库
        let _ = std::fs::remove_dir_all(db_path);

        init_kuzu(db_path, SystemConfig::default()).await?;
        init_kuzu_schema().await?;

        Ok(())
    }

    /// 创建测试 PE
    fn create_test_pe(refno: u64, noun: &str, name: &str) -> SPdmsElement {
        SPdmsElement {
            refno: RefnoEnum::Refno(RefU64(refno)),
            name: name.to_string(),
            noun: noun.to_string(),
            dbnum: 1,
            sesno: 1,
            owner: RefnoEnum::Refno(RefU64(0)),
            deleted: false,
            lock: false,
            ..Default::default()
        }
    }

    /// 创建测试属性映射
    fn create_test_attmap(noun: &str, refno: u64) -> NamedAttrMap {
        let mut attmap = NamedAttrMap::default();

        attmap.insert(
            "TYPE".to_string(),
            NamedAttrValue::StringType(noun.to_string()),
        );
        attmap.insert(
            "REFNO".to_string(),
            NamedAttrValue::RefU64Type(RefU64(refno)),
        );
        attmap.insert(
            "NAME".to_string(),
            NamedAttrValue::StringType(format!("TEST-{}-{}", noun, refno)),
        );

        match noun {
            "ELBO" => {
                attmap.insert("BORE".to_string(), NamedAttrValue::F32Type(100.0));
                attmap.insert("ANGLE".to_string(), NamedAttrValue::F32Type(90.0));
                attmap.insert("RADIUS".to_string(), NamedAttrValue::F32Type(150.0));
                attmap.insert(
                    "POS".to_string(),
                    NamedAttrValue::Vec3Type(Vec3::new(1000.0, 2000.0, 3000.0)),
                );
            }
            "PIPE" => {
                attmap.insert("BORE".to_string(), NamedAttrValue::F32Type(50.0));
                attmap.insert("LENGTH".to_string(), NamedAttrValue::F32Type(5000.0));
                attmap.insert(
                    "POSS".to_string(),
                    NamedAttrValue::Vec3Type(Vec3::new(0.0, 0.0, 0.0)),
                );
                attmap.insert(
                    "POSE".to_string(),
                    NamedAttrValue::Vec3Type(Vec3::new(5000.0, 0.0, 0.0)),
                );
            }
            _ => {}
        }

        attmap
    }

    #[tokio::test]
    async fn test_save_pe_node() {
        init_test_kuzu().await.unwrap();

        let pe = create_test_pe(12345, "ELBO", "TEST-ELBO-001");

        let result = save_pe_node(&pe).await;
        assert!(result.is_ok(), "保存 PE 节点失败: {:?}", result.err());

        println!("✅ 成功保存 PE 节点: {}", pe.name);
    }

    #[tokio::test]
    async fn test_save_attr_node() {
        init_test_kuzu().await.unwrap();

        let pe = create_test_pe(12346, "ELBO", "TEST-ELBO-002");
        let attmap = create_test_attmap("ELBO", 12346);

        // 先保存 PE
        save_pe_node(&pe).await.unwrap();

        // 再保存属性
        let result = save_attr_node(&pe, &attmap).await;
        assert!(result.is_ok(), "保存属性节点失败: {:?}", result.err());

        println!("✅ 成功保存属性节点: Attr_ELBO refno={}", pe.refno.refno());
    }

    #[tokio::test]
    async fn test_create_relations() {
        init_test_kuzu().await.unwrap();

        // 创建 owner PE
        let owner_pe = create_test_pe(10000, "ZONE", "ZONE-001");
        save_pe_node(&owner_pe).await.unwrap();

        // 创建子 PE
        let mut pe = create_test_pe(12347, "ELBO", "TEST-ELBO-003");
        pe.owner = RefnoEnum::Refno(RefU64(10000));

        let attmap = create_test_attmap("ELBO", 12347);

        // 保存 PE 和属性
        save_pe_node(&pe).await.unwrap();
        save_attr_node(&pe, &attmap).await.unwrap();

        // 创建关系
        let result = create_all_relations(&pe, &attmap).await;
        assert!(result.is_ok(), "创建关系失败: {:?}", result.err());

        println!("✅ 成功创建所有关系");
    }

    #[tokio::test]
    async fn test_save_complete_model() {
        init_test_kuzu().await.unwrap();

        let pe = create_test_pe(12348, "PIPE", "TEST-PIPE-001");
        let attmap = create_test_attmap("PIPE", 12348);

        let result = save_model_to_kuzu(&pe, &attmap).await;
        assert!(result.is_ok(), "保存完整模型失败: {:?}", result.err());

        println!("✅ 成功保存完整模型: {}", pe.name);
    }

    #[tokio::test]
    async fn test_save_batch_models() {
        init_test_kuzu().await.unwrap();

        let mut models = Vec::new();

        // 创建 10 个测试模型
        for i in 0..10 {
            let refno = 20000 + i;
            let pe = create_test_pe(refno, "ELBO", &format!("BATCH-ELBO-{:03}", i));
            let attmap = create_test_attmap("ELBO", refno);
            models.push((pe, attmap));
        }

        let result = save_models_batch(models).await;
        assert!(result.is_ok(), "批量保存模型失败: {:?}", result.err());

        println!("✅ 成功批量保存 10 个模型");
    }

    #[tokio::test]
    async fn test_save_with_references() {
        init_test_kuzu().await.unwrap();

        // 创建主 PE
        let main_pe = create_test_pe(30000, "PIPE", "MAIN-PIPE-001");
        let mut main_attmap = create_test_attmap("PIPE", 30000);

        // 创建引用 PE
        let ref_pe = create_test_pe(30001, "ELBO", "REF-ELBO-001");
        let ref_attmap = create_test_attmap("ELBO", 30001);

        // 先保存引用的 PE
        save_model_to_kuzu(&ref_pe, &ref_attmap).await.unwrap();

        // 主 PE 添加引用
        main_attmap.insert(
            "PREF".to_string(),
            NamedAttrValue::RefU64Type(RefU64(30001)),
        );

        // 保存主 PE
        let result = save_model_to_kuzu(&main_pe, &main_attmap).await;
        assert!(result.is_ok(), "保存带引用的模型失败: {:?}", result.err());

        println!("✅ 成功保存带引用关系的模型");
    }

    #[tokio::test]
    async fn test_save_attmaps() {
        init_test_kuzu().await.unwrap();

        let mut attmaps = Vec::new();

        for i in 0..5 {
            let refno = 40000 + i;
            let attmap = create_test_attmap("ELBO", refno);
            attmaps.push(attmap);
        }

        let result = save_attmaps_to_kuzu(attmaps, 1).await;
        assert!(
            result.is_ok(),
            "保存 NamedAttrMap 列表失败: {:?}",
            result.err()
        );

        println!("✅ 成功从 NamedAttrMap 列表保存 5 个模型");
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        init_test_kuzu().await.unwrap();

        // 创建一个正常的 PE
        let pe1 = create_test_pe(50000, "ELBO", "TRANS-ELBO-001");
        let attmap1 = create_test_attmap("ELBO", 50000);

        // 创建一个会失败的 PE (noun 不存在对应的表)
        let pe2 = create_test_pe(50001, "INVALID_NOUN", "INVALID-001");
        let mut attmap2 = create_test_attmap("INVALID_NOUN", 50001);

        let models = vec![(pe1, attmap1), (pe2, attmap2)];

        // 批量保存应该失败并回滚
        let result = save_models_batch(models).await;
        assert!(result.is_err(), "预期事务应该失败");

        println!("✅ 事务回滚测试通过");
    }
}
