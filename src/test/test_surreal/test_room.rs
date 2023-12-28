use crate::{rs_surreal, tool::math_tool};

use std::sync::Arc;
use glam::Mat3;
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_load_rooms() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let tree = rs_surreal::load_aabb_tree()
        .await
        .unwrap();


    Ok(())
}