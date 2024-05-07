use std::collections::{BTreeMap, BTreeSet, HashMap};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::sync::RwLock;
use serde_with::DisplayFromStr;
use crate::{
    accel_tree::acceleration_tree::{AccelerationTree, RStarBoundingBox},
    SUL_DB, RefU64,
};
use std::str::FromStr;
use crate::test::test_surreal::{init_surreal_with_signin, init_test_surreal};

//或者改成第一次，需要去加载，后续就不用了
//启动的时候就要去加载到内存里
pub static GLOBAL_AABB_TREE: Lazy<RwLock<AccelerationTree>> =
    Lazy::new(|| RwLock::new(AccelerationTree::default()));

// 不要每次都加载，需要检查缓存，如果缓存有，就不用从数据库里刷新了
pub async fn load_aabb_tree() -> anyhow::Result<bool> {

    //如果有缓存文件，直接读取缓存文件
    //测试分页查询
    let mut rstar_objs = vec![];
    let mut offset = 0;

    let page_count = 1000;
    loop {
        //需要过滤
        let sql = format!(
            "select in as refno, aabb.d.* as aabb, in.noun as noun from inst_relate where aabb.d!=none and type=0 start {} limit {page_count}",
            offset
        );
        let mut response = SUL_DB.query(&sql).await?;
        let refno_aabbs: Vec<RStarBoundingBox> = response.take(0).unwrap();
        if refno_aabbs.is_empty() {
            break;
        }
        rstar_objs.extend(refno_aabbs);
        offset += page_count;
    }
    dbg!(rstar_objs.len());

    //存储在全局变量里, 每次都重新加载，还是就用数据文件来表达？当做资源来加载，不用每次都去加载
    //加个时间戳，来表达是不是最新的rtree
    let tree = AccelerationTree::load(rstar_objs);
    // tree.serialize_to_bin_file();
    *GLOBAL_AABB_TREE.write().await = tree;

    Ok(true)
}

//计算单个房间包含哪些构件
async fn calculate_room(room_refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    // let mut withing_room_items = vec![];

    //先获得房间的包围盒

    //所有元件对应的关键点的获取

    //做一个 relate 表达这种房间关系，分为两种情况，内含 contains 和相交 intersection


    // if let Some(room_abb) = info.aabb {
    //     withing_room_items = rtree
    //         .locate_intersecting_bounds(&room_abb)
    //         .collect::<Vec<_>>();

    //     let hashes = inst_geos.iter().map(|x| x.geo_hash).collect::<Vec<_>>();
    //     let room_mesh_mgr = query_pdms_mesh_aql(&database, hashes.iter())
    //         .await
    //         .unwrap_or_default();
    //     for (&hash, geo) in hashes.iter().zip(inst_geos) {
    //         if let Some(room_mesh) = room_mesh_mgr.get_mesh(hash) {
    //             let t = info.get_geo_world_transform(geo);
    //             let collider_mesh = room_mesh.get_tri_mesh(t.compute_matrix());
    //             let mut outer_refnos = vec![];
    //             //需要批量去获取数据

    //             for (refno, aabb) in &withing_room_items {
    //                 //检查目标的坐标点不在它自身包围盒的情况，这种就需要用相交的算法去计算
    //                 //check 是否包含在房间内
    //                 let contain_point = match collider_mesh.cast_local_ray_and_get_normal(
    //                     &Ray::new(aabb.center(), Vector::new(0.0, 0.0, 1.0)),
    //                     100000.0,
    //                     false,
    //                 ) {
    //                     Some(intersection) => collider_mesh.is_backface(intersection.feature),
    //                     None => false,
    //                 };
    //                 if !contain_point {
    //                     outer_refnos.push(*refno);
    //                 }
    //                 //如果是风管，就需要这么去检测是否发生碰撞
    //                 //后续需要用包围盒再去判断一次
    //                 // collider_mesh.intersection_with_aabb();
    //             }

    //             //排除room的类型
    //             withing_room_items
    //                 .retain(|(refno, _)| !outer_refnos.contains(refno) && *refno != room_refno);

    //             // dbg!(&withing_room_refnos);
    //         }
    //     }
    //     //再次过滤room，通过判断位置是否在room的mesh里来判断
    // }

    return Ok(Vec::new());
    // return Ok(withing_room_items.iter().map(|x| x.0).collect());
}

// //计算所有房间包含的其他参考号
// pub async fn calculate_rooms() -> anyhow::Result<()> {
//     let rtree = self
//         .rtree
//         .as_ref()
//         .ok_or(anyhow::anyhow!("空间树未生成。"))?;
//     let database = self.get_arango_db().await?;
//     //指定哪个site下有房间节点
//     let Some(room_root_refnos) = &self.db_option.room_root_refnos else {
//         return Ok(());
//     };

//     let mut room_eles_map: HashMap<RefU64, (Aabb, Vec<RefU64>)> = HashMap::new();
//     let mut room_panels_map: HashMap<RefU64, Vec<RoomPanelElement>> = HashMap::new();
//     for r in room_root_refnos {
//         let Ok(room_root_refno) = RefU64::from_str(r) else {
//             continue;
//         };
//         let room_panels =
//             query_deep_children_refnos_fuzzy(&database, &[room_root_refno], &["PANE"]).await?;
//         //以panel的owner为房间的参考号
//         println!("房间下的panel数量为: {}", room_panels.len());
//         let inst_data = query_insts_shape_data(
//             &database,
//             &room_panels,
//             Some(&[GeoBasicType::Pos, GeoBasicType::Compound]),
//         )
//         .await?;
//         for (panel_refno, info) in &inst_data.inst_info_map {
//             let Some(inst_geos) = inst_data.get_inst_geos(info) else {
//                 continue;
//             };
//             let Some(aabb) = info.aabb else {
//                 continue;
//             };
//             let r = self.calculate_room(info, inst_geos, rtree).await?;
//             let room_refno = self.get_owner(info.refno);
//             let room_panel_ele = RoomPanelElement {
//                 refno: *panel_refno,
//                 aabb,
//                 inst_geo: inst_geos.first().cloned().unwrap_or_default(),
//                 transform: info.world_transform,
//             };
//             if let Some((room_aabb, refnos)) = room_eles_map.get_mut(&room_refno) {
//                 room_aabb.merge(&aabb);
//                 refnos.extend_from_slice(&r);
//                 room_panels_map
//                     .get_mut(&room_refno)
//                     .unwrap()
//                     .push(room_panel_ele);
//             } else {
//                 room_eles_map.insert(room_refno, (aabb, r));
//                 room_panels_map.insert(room_refno, vec![room_panel_ele]);
//             }
//         }
//         println!("房间内元件的数量为：{}", room_eles_map.len());
//     }

//     self.save_room_info_to_arangodb(room_eles_map, room_panels_map)
//         .await?;
//     Ok(())
// }

/// 查询项目的所有房间
///
/// 返回值: k 厂房 v 房间编号
pub async fn query_all_room_name() -> anyhow::Result<HashMap<String, BTreeSet<String>>> {
    let mut map = HashMap::new();
    let mut response = SUL_DB
        .query(include_str!("schemas/query_all_room.surql"))
        .await?;
    let results: Vec<Vec<String>> = response.take(0)?;
    for r in results.clone() {
        for room in r {
            let split = room.split("-").collect::<Vec<_>>();
            let Some(first) = split.first() else { continue; };
            let Some(last) = split.last() else { continue; };
            if !match_room_name(last) { continue; };
            map.entry(first[1..].to_string()).or_insert_with(BTreeSet::new).insert(last.to_string());
        }
    }
    Ok(map)
}

/// 查询多个refno所属的房间号，bran和equi也适用
pub async fn query_room_name_from_refnos(owner: Vec<RefU64>) -> anyhow::Result<HashMap<RefU64, String>> {
    #[serde_as]
    #[derive(Debug, Serialize, Deserialize)]
    struct RoomNameQueryRequest {
        #[serde_as(as = "DisplayFromStr")]
        pub id: RefU64,
        pub room: Option<String>,
    }

    let owners = owner.into_iter().map(|o| o.to_pe_key()).collect::<Vec<String>>();
    let sql = format!("select id,fn::room_code(id)[0] as room from {}", serde_json::to_string(&owners).unwrap_or("[]".to_string()));
    let mut response = SUL_DB
        .query(sql)
        .await?;
    let result: Vec<RoomNameQueryRequest> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| (x.id, x.room.unwrap_or("".to_string()))).collect::<HashMap<RefU64, String>>();
    Ok(r)
}

/// 查找设备和阀门所属的楼板
pub async fn query_equi_or_valv_belong_floors(refnos: Vec<RefU64>) -> anyhow::Result<HashMap<RefU64, (String, f32)>> {
    #[serde_as]
    #[derive(Serialize, Deserialize, Debug)]
    struct BelongFloorResponse {
        #[serde_as(as = "DisplayFromStr")]
        pub id: RefU64,
        pub floor: Option<String>,
        pub height: Option<f32>,
    }

    let refnos = refnos.into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let request = serde_json::to_string(&refnos)?;
    let sql = format!("select id,(->nearest_relate.out.REFNO)[0] as floor,(->nearest_relate.dist)[0] as height from {}", request);
    let mut response = SUL_DB
        .query(sql)
        .await?;
    let result: Vec<BelongFloorResponse> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| (x.id,
                  (x.floor.map_or("".to_string(), |x| RefU64::from_str(&x).unwrap().to_pdms_str()),
                   x.height.unwrap_or(0.0))))
        .collect::<HashMap<RefU64, (String, f32)>>();
    Ok(r)
}

/// 正则匹配是否满足房间命名规则
fn match_room_name(room_name: &str) -> bool {
    let regex = Regex::new(r"^[A-Z]\d{3}$").unwrap();
    regex.is_match(room_name)
}

#[tokio::test]
async fn test_query_all_room_name() {
    init_test_surreal().await;
    let r = query_all_room_name().await.unwrap();
    dbg!(&r);
}