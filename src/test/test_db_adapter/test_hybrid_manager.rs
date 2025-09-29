#[cfg(test)]
mod tests {
    use crate::db_adapter::{
        DatabaseAdapter, HybridConfig, HybridDatabaseManager, HybridMode, QueryContext,
        SurrealAdapter,
    };
    use crate::types::{RefnoEnum, RefU64};
    use std::sync::Arc;

    #[cfg(feature = "kuzu")]
    use crate::db_adapter::KuzuAdapter;
    #[cfg(feature = "kuzu")]
    use crate::rs_kuzu::{init_kuzu, init_kuzu_schema};
    #[cfg(feature = "kuzu")]
    use kuzu::SystemConfig;

    fn setup_surreal_only() -> HybridDatabaseManager {
        let surreal = Arc::new(SurrealAdapter::new());
        let config = HybridConfig {
            mode: HybridMode::SurrealPrimary,
            query_timeout_ms: 5000,
            fallback_on_error: true,
            enable_cache: false,
            cache_ttl_secs: 300,
        };

        HybridDatabaseManager::new(surreal, None, config)
    }

    #[cfg(feature = "kuzu")]
    async fn setup_dual_database() -> HybridDatabaseManager {
        let kuzu_config = SystemConfig::default();
        init_kuzu("test_kuzu_hybrid.db", kuzu_config)
            .await
            .expect("Failed to initialize Kuzu");
        init_kuzu_schema().await.expect("Failed to initialize schema");

        let surreal = Arc::new(SurrealAdapter::new("test_surreal".to_string()));
        let kuzu = Arc::new(KuzuAdapter::new("test_kuzu".to_string()));

        let config = HybridConfig {
            mode: HybridMode::DualSurrealPreferred,
            query_timeout_ms: 5000,
            fallback_on_error: true,
            enable_cache: false,
            cache_ttl_secs: 300,
        };

        HybridDatabaseManager::new(surreal, Some(kuzu), config, "test_hybrid_dual".to_string())
    }

    #[tokio::test]
    async fn test_hybrid_manager_name() {
        let manager = setup_surreal_only();
        assert!(manager.name().contains("Hybrid"));
    }

    #[tokio::test]
    async fn test_hybrid_manager_capabilities() {
        let manager = setup_surreal_only();
        let caps = manager.capabilities();

        assert!(caps.supports_graph_traversal);
        assert!(caps.supports_transactions);
    }

    #[tokio::test]
    async fn test_hybrid_manager_health_check() {
        let manager = setup_surreal_only();
        let result = manager.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_hybrid_manager_get_pe() {
        let manager = setup_surreal_only();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = manager.get_pe(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hybrid_manager_query_children() {
        let manager = setup_surreal_only();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = manager.query_children(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hybrid_manager_get_attmap() {
        let manager = setup_surreal_only();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = manager.get_attmap(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[cfg(feature = "kuzu")]
    #[tokio::test]
    async fn test_dual_database_fallback() {
        let manager = setup_dual_database().await;
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = manager.get_pe(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[cfg(feature = "kuzu")]
    #[tokio::test]
    async fn test_dual_database_graph_query() {
        let manager = setup_dual_database().await;
        let ctx = QueryContext {
            requires_graph_traversal: true,
            ..Default::default()
        };

        let from = RefnoEnum::from(RefU64::from(1u64));
        let to = RefnoEnum::from(RefU64::from(10u64));
        let result = manager.shortest_path(from, to, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hybrid_modes() {
        let surreal = Arc::new(SurrealAdapter::new("test_surreal".to_string()));

        let modes = vec![
            HybridMode::SurrealPrimary,
            HybridMode::DualSurrealPreferred,
        ];

        for mode in modes {
            let config = HybridConfig {
                mode,
                query_timeout_ms: 5000,
                fallback_on_error: true,
                enable_cache: false,
                cache_ttl_secs: 300,
            };

            let manager = HybridDatabaseManager::new(
                surreal.clone(),
                None,
                config,
                "test_modes".to_string(),
            );

            let result = manager.health_check().await;
            assert!(result.is_ok());
        }
    }
}