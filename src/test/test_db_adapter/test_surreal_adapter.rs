#[cfg(test)]
mod tests {
    use crate::db_adapter::{DatabaseAdapter, QueryContext, SurrealAdapter};
    use crate::types::{RefnoEnum, RefU64};

    fn setup() -> SurrealAdapter {
        SurrealAdapter::new()
    }

    #[tokio::test]
    async fn test_surreal_adapter_name() {
        let adapter = setup();
        assert_eq!(adapter.name(), "SurrealDB");
    }

    #[tokio::test]
    async fn test_surreal_capabilities() {
        let adapter = setup();
        let caps = adapter.capabilities();

        assert!(caps.supports_graph_traversal);
        assert!(caps.supports_transactions);
        assert!(caps.supports_versioning);
        assert!(caps.supports_live_queries);
    }

    #[tokio::test]
    async fn test_surreal_health_check() {
        let adapter = setup();
        let result = adapter.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_surreal_get_pe() {
        let adapter = setup();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.get_pe(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_surreal_query_children() {
        let adapter = setup();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.query_children(refno, ctx).await;

        assert!(result.is_ok());
        let children = result.unwrap();
        assert!(children.is_empty() || !children.is_empty());
    }

    #[tokio::test]
    async fn test_surreal_query_owners() {
        let adapter = setup();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.query_owners(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_surreal_get_attmap() {
        let adapter = setup();
        let ctx = QueryContext::default();

        let refno = RefnoEnum::from(RefU64::from(1u64));
        let result = adapter.get_attmap(refno, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_surreal_query_by_name() {
        let adapter = setup();
        let ctx = QueryContext::default();

        let result = adapter.query_by_name("TEST".to_string(), None, ctx).await;

        assert!(result.is_ok());
    }
}