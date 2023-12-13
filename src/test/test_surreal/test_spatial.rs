use crate::{rs_surreal, tool::math_tool};

use std::sync::Arc;
use glam::Mat3;
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let transform = rs_surreal::get_world_transform("25688/7960".into())
        .await
        .unwrap().unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    dbg!(&ori_str);
    Ok(())
}