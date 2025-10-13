//! Kuzu 查询集成测试
//!
//! 验证 Kuzu 查询结果与 SurrealDB 的一致性

#[cfg(all(test, feature = "kuzu"))]
mod kuzu_query_tests {
    use crate::init_surreal;
    use crate::rs_kuzu::queries::hierarchy as kuzu_hierarchy;
    use crate::rs_kuzu::queries::type_filter as kuzu_type_filter;
    use crate::rs_kuzu::query_router::{QueryEngine, QueryRouter};
    use crate::rs_kuzu::*;
    use crate::rs_surreal;
    use crate::rs_surreal::graph as surreal_graph;
    use crate::rs_surreal::mdb as surreal_mdb;
    use crate::types::*;
    use kuzu::SystemConfig;
    use std::collections::HashSet;

    /// 初始化测试环境
    async fn setup_test_env() -> anyhow::Result<()> {
        // 初始化 SurrealDB
        init_surreal().await?;

        // 初始化 Kuzu
        let kuzu_path = "./test_output/kuzu_1112_comparison.db";
        if std::path::Path::new(kuzu_path).exists() {
            init_kuzu(kuzu_path, SystemConfig::default()).await?;
        } else {
            eprintln!("⚠️  警告: Kuzu 数据库不存在，跳过 Kuzu 测试");
        }

        Ok(())
    }

    /// 获取测试用的 refno
    async fn get_test_refnos() -> anyhow::Result<Vec<RefnoEnum>> {
        // 查询一些有子节点的元素用于测试
        let refnos =
            surreal_mdb::query_type_refnos_by_dbnum(&["ZONE"], 1112, Some(true), false).await?;
        Ok(refnos.into_iter().take(5).collect())
    }

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_children_query_consistency() {
        setup_test_env().await.unwrap();
        let test_refnos = get_test_refnos().await.unwrap();

        for refno in test_refnos {
            // SurrealDB 查询
            let surreal_result = rs_surreal::get_children_refnos(refno).await.unwrap();

            // Kuzu 查询
            let kuzu_result = kuzu_hierarchy::kuzu_get_children_refnos(refno)
                .await
                .unwrap();

            // 转换为 HashSet 进行比较（忽略顺序）
            let surreal_set: HashSet<_> = surreal_result.iter().collect();
            let kuzu_set: HashSet<_> = kuzu_result.iter().collect();

            println!(
                "refno {:?}: SurrealDB={}, Kuzu={}",
                refno,
                surreal_result.len(),
                kuzu_result.len()
            );

            // 验证数量一致
            assert_eq!(
                surreal_set.len(),
                kuzu_set.len(),
                "子节点数量不一致 for refno {:?}",
                refno
            );

            // 验证内容一致
            assert_eq!(
                surreal_set, kuzu_set,
                "子节点内容不一致 for refno {:?}",
                refno
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_deep_children_query_consistency() {
        setup_test_env().await.unwrap();
        let test_refnos = get_test_refnos().await.unwrap();

        for refno in test_refnos.into_iter().take(2) {
            // SurrealDB 查询
            let surreal_result = surreal_graph::query_deep_children_refnos(refno)
                .await
                .unwrap();

            // Kuzu 查询
            let kuzu_result = kuzu_hierarchy::kuzu_query_deep_children_refnos(refno)
                .await
                .unwrap();

            let surreal_set: HashSet<_> = surreal_result.iter().collect();
            let kuzu_set: HashSet<_> = kuzu_result.iter().collect();

            println!(
                "refno {:?}: SurrealDB={}, Kuzu={}",
                refno,
                surreal_result.len(),
                kuzu_result.len()
            );

            assert_eq!(
                surreal_set.len(),
                kuzu_set.len(),
                "深层子节点数量不一致 for refno {:?}",
                refno
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_type_filter_query_consistency() {
        setup_test_env().await.unwrap();

        let test_cases = vec![
            (vec!["PIPE"], 1112),
            (vec!["EQUI"], 1112),
            (vec!["ZONE"], 1112),
        ];

        for (nouns, dbnum) in test_cases {
            let nouns_ref: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();

            // SurrealDB 查询
            let surreal_result =
                surreal_mdb::query_type_refnos_by_dbnum(&nouns_ref, dbnum, None, false)
                    .await
                    .unwrap();

            // Kuzu 查询
            let kuzu_result =
                kuzu_type_filter::kuzu_query_type_refnos_by_dbnum(&nouns_ref, dbnum, None)
                    .await
                    .unwrap();

            let surreal_set: HashSet<_> = surreal_result.iter().collect();
            let kuzu_set: HashSet<_> = kuzu_result.iter().collect();

            println!(
                "nouns {:?}: SurrealDB={}, Kuzu={}",
                nouns,
                surreal_result.len(),
                kuzu_result.len()
            );

            assert_eq!(
                surreal_set.len(),
                kuzu_set.len(),
                "类型过滤结果数量不一致 for nouns {:?}",
                nouns
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_router_auto_mode() {
        setup_test_env().await.unwrap();
        let test_refnos = get_test_refnos().await.unwrap();

        let router = QueryRouter::new(QueryEngine::Auto);

        for refno in test_refnos.into_iter().take(3) {
            // 使用路由器查询（应该自动选择 Kuzu，失败则回退到 SurrealDB）
            let result = router.get_children_refnos(refno).await;
            assert!(result.is_ok(), "路由器查询失败 for refno {:?}", refno);

            let children = result.unwrap();
            println!("refno {:?}: 路由器返回 {} 个子节点", refno, children.len());
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_router_kuzu_mode() {
        setup_test_env().await.unwrap();
        let test_refnos = get_test_refnos().await.unwrap();

        let router = QueryRouter::new(QueryEngine::Kuzu);

        for refno in test_refnos.into_iter().take(3) {
            let result = router.get_children_refnos(refno).await;
            assert!(result.is_ok(), "Kuzu 查询失败 for refno {:?}", refno);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_router_surrealdb_mode() {
        setup_test_env().await.unwrap();
        let test_refnos = get_test_refnos().await.unwrap();

        let router = QueryRouter::new(QueryEngine::SurrealDB);

        for refno in test_refnos.into_iter().take(3) {
            let result = router.get_children_refnos(refno).await;
            assert!(result.is_ok(), "SurrealDB 查询失败 for refno {:?}", refno);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_filter_deep_children_consistency() {
        setup_test_env().await.unwrap();
        let test_refnos = get_test_refnos().await.unwrap();

        let filter_nouns = ["PIPE", "EQUI"];

        for refno in test_refnos.into_iter().take(2) {
            // SurrealDB 查询
            let surreal_result = surreal_graph::query_filter_deep_children(refno, &filter_nouns)
                .await
                .unwrap();

            // Kuzu 查询
            let kuzu_result = kuzu_hierarchy::kuzu_query_filter_deep_children(refno, &filter_nouns)
                .await
                .unwrap();

            let surreal_set: HashSet<_> = surreal_result.iter().collect();
            let kuzu_set: HashSet<_> = kuzu_result.iter().collect();

            println!(
                "refno {:?} with filter {:?}: SurrealDB={}, Kuzu={}",
                refno,
                filter_nouns,
                surreal_result.len(),
                kuzu_result.len()
            );

            assert_eq!(
                surreal_set.len(),
                kuzu_set.len(),
                "过滤深层子节点数量不一致 for refno {:?}",
                refno
            );
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_ancestor_query_consistency() {
        setup_test_env().await.unwrap();

        // 查询一些深层节点来测试祖先查询
        let deep_refnos = surreal_mdb::query_type_refnos_by_dbnum(&["PIPE"], 1112, None, false)
            .await
            .unwrap();

        for refno in deep_refnos.into_iter().take(5) {
            // SurrealDB 查询
            let surreal_result = crate::query_ancestor_refnos(refno).await.unwrap();

            // Kuzu 查询
            let kuzu_result = kuzu_hierarchy::kuzu_query_ancestor_refnos(refno)
                .await
                .unwrap();

            let surreal_set: HashSet<_> = surreal_result.iter().collect();
            let kuzu_set: HashSet<_> = kuzu_result.iter().collect();

            println!(
                "refno {:?}: SurrealDB ancestors={}, Kuzu ancestors={}",
                refno,
                surreal_result.len(),
                kuzu_result.len()
            );

            // 祖先数量应该一致
            assert_eq!(
                surreal_set.len(),
                kuzu_set.len(),
                "祖先数量不一致 for refno {:?}",
                refno
            );
        }
    }

    #[test]
    fn test_query_engine_config() {
        use crate::rs_kuzu::query_router::{get_query_engine, set_query_engine};

        // 测试设置和获取全局配置
        set_query_engine(QueryEngine::Kuzu);
        assert_eq!(get_query_engine(), QueryEngine::Kuzu);

        set_query_engine(QueryEngine::SurrealDB);
        assert_eq!(get_query_engine(), QueryEngine::SurrealDB);

        set_query_engine(QueryEngine::Auto);
        assert_eq!(get_query_engine(), QueryEngine::Auto);
    }
}
