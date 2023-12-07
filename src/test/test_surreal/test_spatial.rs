use crate::rs_surreal;

use std::sync::Arc;
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let transform = rs_surreal::get_world_transform("17496_105912".into())
        .await
        .unwrap();
    dbg!(transform);
    Ok(())
}