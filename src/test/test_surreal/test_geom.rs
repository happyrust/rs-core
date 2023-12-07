use crate::rs_surreal;

use std::{sync::Arc, time::Instant};
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_query_inst_refnos() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let mut time = Instant::now();
    let refnos = rs_surreal::query_deep_inst_info_refnos("17496_248588".into())
        .await
        .unwrap();
    dbg!(time.elapsed().as_secs_f32());
    // dbg!(refnos);
    Ok(())
}