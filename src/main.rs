// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     //创建保存元数据里的空间关系结果
//     init_test_surreal().await;
//     // load_aabb_tree().await.unwrap();
//     // create_valve_floor_relations().await.unwrap();
//     dbg!(db1_hash("FJFD"));
//     dbg!(db1_hash("DAMP"));
//     dbg!(db1_hash("MESH"));
//     dbg!(db1_dehash(642952140));
//     // get_uda_type_refnos_from_select_refnos(vec![RefU64::from_str("")])
//     Ok(())
// }

#[test]
fn get_noun_hash() {
    use aios_core::tool::db_tool::{db1_dehash, db1_hash};

    let noun = "SCOM";
    let hash = db1_hash(noun);
    dbg!(hash);
    let noun = "WELDTY";
    let hash = db1_hash(noun);
    dbg!(hash);
    let hashes = [534980, 531454, 369970574];
    for hash in hashes {
        let str = db1_dehash(hash);
        dbg!(&hash);
        dbg!(str);
    }
}

fn main() {}
