use crate::RefU64;

///收集所有的房间元素，按一定规则查找出房间
pub async fn gather_rooms() -> anyhow::Result<()> {

    Ok(())
}

///计算单个房间包含哪些构件
pub async fn calculate_room(room_refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    // let mut withing_room_items = vec![];
    //房间是有多个 pannel，需要遍历

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