use crate::rs_surreal;

use std::sync::Arc;
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    // let refno: RefU64 = "17496_107068".into();
    // let mgr = get_test_ams_db_manager_async().await;
    // let transform = mgr.get_world_transform(refno).await.unwrap().unwrap();
    // dbg!(&transform);

    let pe = rs_surreal::get_pe("17496_107068".into())
        .await
        .unwrap();
    dbg!(pe);
    Ok(())
}
