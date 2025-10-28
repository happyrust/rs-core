use crate::{SUL_DB, SurrealQueryExt, rs_surreal, tool::math_tool};

use crate::room::data::RoomElement;
use glam::Mat3;
use std::sync::Arc;
use surrealdb::types::RecordId;

#[tokio::test]
async fn test_load_rooms() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    // let tree = rs_surreal::load_aabb_tree()
    //     .await
    //     .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_save_room_data() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let d = RoomElement {
        name: "ROOM-1".to_string(),
        ..Default::default()
    };
    SUL_DB
        .query_response(format!(
            "insert ignore into room_ele {}",
            serde_json::to_string(&d).unwrap()
        ))
        .await
        .unwrap();

    Ok(())
}
