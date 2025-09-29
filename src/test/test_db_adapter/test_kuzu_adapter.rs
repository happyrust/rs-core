#[cfg(all(test, feature = "kuzu"))]
mod tests {
    use crate::db_adapter::{DatabaseAdapter, KuzuAdapter, QueryContext};
    use crate::rs_kuzu::{init_kuzu, init_kuzu_schema};
    use crate::types::{RefnoEnum, RefU64};
    use kuzu::SystemConfig;

    async fn setup() -> KuzuAdapter {
        let config = SystemConfig::default();
        init_kuzu("test_kuzu.db", config)
            .await
            .expect("Failed to initialize Kuzu");
        init_kuzu_schema().await.expect("Failed to initialize schema");

        KuzuAdapter::new("test_kuzu".to_string())
    }

    #[tokio::test]
    async fn test_kuzu_adapter_name() {
        let adapter = setup().await;
        assert_eq!(adapter.name(), "test_kuzu");
    }

    #[tokio::test]
    async fn test_kuzu_capabilities() {
        let adapter = setup().await;
        let caps = adapter.capabilities();

        assert!(caps.supports_graph_traversal);
        assert!(caps.supports_transactions);
        assert!(!caps.supports_versioning);
        assert!(!caps.supports_live_queries);
        assert!(caps.supports_full_text_search);
        assert!(caps.supports_vector_index);
    }

    #[tokio::test]
    async fn test_kuzu_health_check() {
        let adapter = setup().await;
        let result = adapter.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_kuzu_get_pe() {
        let adapter = setup().await;
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.get_pe(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kuzu_query_children() {
        let adapter = setup().await;
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.query_children(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kuzu_shortest_path() {
        let adapter = setup().await;
        let ctx = QueryContext {
            requires_graph_traversal: true,
            ..Default::default()
        };

        let from = RefnoEnum::from(RefU64::from(1u64));
        let to = RefnoEnum::from(RefU64::from(10u64));
        let result = adapter.shortest_path(from, to, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kuzu_query_subtree() {
        let adapter = setup().await;
        let ctx = QueryContext {
            requires_graph_traversal: true,
            ..Default::default()
        };

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.query_subtree(refno, Some(3), ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_kuzu_get_attmap() {
        let adapter = setup().await;
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.get_attmap(refno, ctx).await;

        assert!(result.is_ok());
    }
}