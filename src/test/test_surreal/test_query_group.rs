use crate::{RefU64, rs_surreal};
use glam::Vec3;
use std::sync::Arc;

#[tokio::test]
async fn test_group_cata_hash() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    // let refnos: Vec<RefU64> = vec!["15302_2194".into()];
    let refnos: Vec<RefU64> = vec!["24381_47118".into()];
    let group = rs_surreal::query_group_by_cata_hash(&refnos)
        .await
        .unwrap();
    dbg!(&group);
    Ok(())
}
