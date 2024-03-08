use crate::{room::room::load_aabb_tree, rs_surreal, tool::math_tool};

use std::sync::Arc;
use glam::{Mat3, Vec3};
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let transform = rs_surreal::get_world_transform("24381_177458".into())
        .await
        .unwrap().unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    dbg!(&ori_str);


    let transform = rs_surreal::get_world_transform("24381_177459".into())
        .await
        .unwrap().unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    dbg!(&ori_str);
    Ok(())
}


#[tokio::test]
async fn test_query_nearest_along() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    load_aabb_tree().await.unwrap();
    let nearest = rs_surreal::query_neareast_along_axis("24383/66745".into(), Vec3::NEG_Z, "FLOOR")
        .await
        .unwrap();
    dbg!(nearest);
    assert_eq!(nearest.to_string().as_str(), "25688_71674");

    let nearest = rs_surreal::query_neareast_along_axis("24383/66771".into(), Vec3::NEG_Z, "FLOOR")
        .await
        .unwrap();
    dbg!(nearest);
    assert_eq!(nearest.to_string(), "25688_45314");
    Ok(())
}