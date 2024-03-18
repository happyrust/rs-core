use aios_core::spatial::pipe::create_valve_floor_relations;
use aios_core::tool::math_tool::*;
use aios_core::{
    load_aabb_tree,
    test::test_surreal::init_test_surreal,
    tool::{
        db_tool::{db1_dehash, db1_hash},
        dir_tool::parse_ori_str_to_quat,
    },
};
use anyhow::Ok;
use glam::{Mat3, Quat};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //创建保存元数据里的空间关系结果
    init_test_surreal().await;
    load_aabb_tree().await.unwrap();
    create_valve_floor_relations().await.unwrap();

    Ok(())
}

#[test]
fn get_noun_hash() {
    let noun = "UDA";
    let hash = db1_hash(noun);
    dbg!(hash);
    let hashes = [582073, 640481, 919399];
    for hash in hashes {
        let str = db1_dehash(hash);
        dbg!(&hash);
        dbg!(str);
    }
}

//todo 提供一个 http 的接口返回数据吗？