use crate::basic::aabb::ParryAabb;
use crate::pdms_types::PdmsGenericType;
use crate::{RefU64, RefnoEnum, SUL_DB, SurlValue, get_inst_relate_keys};
use anyhow::Context;
use bevy_transform::components::Transform;
use chrono::{DateTime, Local, NaiveDateTime};
use glam::{DVec3, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TubiInstQuery {
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    pub old_refno: Option<RefnoEnum>,
    pub generic: Option<String>,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub geo_hash: String,
    pub date: Option<surrealdb::types::Datetime>,
}

fn decode_values<T: DeserializeOwned>(values: Vec<SurlValue>) -> anyhow::Result<Vec<T>> {
    values
        .into_iter()
        .map(|value| {
            let json = value.into_json_value();
            serde_json::from_value(json).context("failed to deserialize Surreal value")
        })
        .collect()
}

pub async fn query_tubi_insts_by_brans(
    bran_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<TubiInstQuery>> {
    let pes: String = bran_refnos
        .iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        r#"
             select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner.noun as generic, aabb.d as world_aabb, world_trans.d as world_trans,
                record::id(out) as geo_hash,
                fn::ses_date(in.id) as date
             from  array::flatten([{}]->tubi_relate) where leave.id != none and aabb.d != none
             "#,
        pes
    );
    // println!("Query tubi insts: {}", &sql);
    let mut response = SUL_DB.query(&sql).await?;
    // dbg!(&response);

    let values: Vec<SurlValue> = response.take(0)?;
    let r = decode_values(values)?;
    Ok(r)
}

pub async fn query_tubi_insts_by_flow(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<TubiInstQuery>> {
    let pes: String = refnos
        .iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        r#"
        array::group(array::complement(select value
        (select in.id as refno, in.owner.noun as generic, aabb.d as world_aabb, world_trans.d as world_trans, record::id(out) as geo_hash,
            fn::ses_date(in.id) as date
            from tubi_relate where leave=$parent.id or arrive=$parent.id)
                from [{}] where in.id != none and  owner.noun in ['BRAN', 'HANG'], [none]))
             "#,
        pes
    );
    // println!("Sql query_tubi_insts_by_flow: {}", &sql);
    let mut response = SUL_DB.query(sql).await?;

    let values: Vec<SurlValue> = response.take(0)?;
    let r = decode_values(values)?;
    Ok(r)
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ModelHashInst {
    pub geo_hash: String,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub is_tubi: bool,
}

#[derive(Debug)]
pub struct ModelInstData {
    pub owner: RefnoEnum,
    pub old_refno: Option<RefnoEnum>,
    pub has_neg: bool,
    pub insts: Vec<ModelHashInst>,
    pub generic: PdmsGenericType,
    pub world_trans: Transform,
    pub world_aabb: ParryAabb,
    pub ptset: Vec<Vec3>,
    pub is_bran_tubi: bool,
    pub date: NaiveDateTime,
}

///
/// 几何实例查询结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct GeomInstQuery {
    /// 构件编号，别名为id
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 历史构件编号
    pub old_refno: Option<RefnoEnum>,
    /// 所属构件编号
    pub owner: RefnoEnum,
    /// 世界坐标系下的包围盒
    pub world_aabb: Aabb,
    /// 世界坐标系下的变换矩阵
    pub world_trans: Transform,
    /// 几何实例列表
    pub insts: Vec<ModelHashInst>,
    /// 是否包含负实体
    pub has_neg: bool,
    /// 构件类型
    pub generic: String,
    /// 点集数据
    pub pts: Option<Vec<Vec3>>,
    /// 时间戳
    pub date: Option<surrealdb::types::Datetime>,
}

/// 几何点集查询结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct GeomPtsQuery {
    /// 构件编号，别名为id
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 世界坐标系下的变换矩阵
    pub world_trans: Transform,
    /// 世界坐标系下的包围盒
    pub world_aabb: Aabb,
    /// 点集组，每组包含一个变换矩阵和可选的点集数据
    pub pts_group: Vec<(Transform, Option<Vec<DVec3>>)>,
}

/// 根据最新refno查询最新insts
/// 根据构件编号查询几何实例信息
///
/// # 参数
///
/// * `refnos` - 构件编号迭代器
/// * `enable_holes` - 是否启用孔洞查询
///
/// # 返回值
///
/// 返回几何实例查询结果的向量
pub async fn query_insts(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();

    //需要区分历史模型和当前最新模型

    let inst_keys = get_inst_relate_keys(&refnos);

    let sql = if enable_holes {
        format!(
            r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
                if booled_id != none {{ [{{ "geo_hash": booled_id }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none && geo_type='Pos')  }} as insts,
                booled_id != none as has_neg,
                dt as date
            from {inst_keys} where aabb.d != none
        "#
        )
    } else {
        format!(
            r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
                (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none && geo_type='Pos') as insts,
                booled_id != none as has_neg,
                dt as date
            from {inst_keys} where aabb.d != none "#
        )
    };
    // println!("Query insts sql: {}", &sql);
    let mut response = SUL_DB.query(sql).await?;
    let values: Vec<SurlValue> = response.take(0)?;
    let mut geom_insts: Vec<GeomInstQuery> = decode_values(values)?;
    // dbg!(&geom_insts);

    Ok(geom_insts)
}

// 根据历史refno查询历史insts
// (legacy implementation removed during SurrealDB v3 migration)

// todo 生成一个测试案例
// pub async fn query_history_insts(
//     refnos: impl IntoIterator<Item = &(RefnoEnum, u32)>,
// ) -> anyhow::Result<Vec<GeomInstQuery>> {
//     let history_inst_keys = refnos
//         .into_iter()
//         .map(|x| format!("inst_relate:{}_{}", x.0, x.1))
//         .collect::<Vec<_>>()
//         .join(",");

//     //todo 如果是ngmr relate, 也要测试一下有没有问题
//     //ngmr relate 的关系可以直接在inst boolean 做这个处理，不需要单独开方法
//     //ngmr的负实体最后再执行
//     let sql = format!(
//         r#"
//     select in.id as refno, in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
//             if (in<-neg_relate)[0] != none && $parent.booled {{ [{{ "geo_hash": record::id(in.id) }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && trans.d != none && geo_type='Pos')  }} as insts
//             from {history_inst_keys} where aabb.d != none
//             "#
//     );
//     // println!("Query insts: {}", &sql);
//     let mut response = SUL_DB.query(sql).await?;
//     let mut geom_insts: Vec<GeomInstQuery> = response.take(0).unwrap();

//     Ok(geom_insts)
// }

/// 根据区域编号查询几何实例信息
///
/// # 参数
///
/// * `refnos` - 区域编号迭代器
/// * `enable_holes` - 是否启用孔洞查询
///
/// # 返回值
///
/// 返回几何实例查询结果的向量
pub async fn query_insts_by_zone(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    let zone_refnos = refnos
        .into_iter()
        .map(|x| format!("ZONE:{}", x))
        .collect::<Vec<_>>()
        .join(",");

    let sql = if enable_holes {
        format!(
            r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
                if booled_id != none {{ [{{ "geo_hash": booled_id }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none && geo_type='Pos')  }} as insts,
                booled_id != none as has_neg,
                fn::ses_date(in.id) as date
            from inst_relate where zone_refno in [{}] and aabb.d != none
            "#,
            zone_refnos
        )
    } else {
        format!(
            r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
                (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none && geo_type='Pos') as insts,
                booled_id != none as has_neg,
                fn::ses_date(in.id) as date
            from inst_relate where zone_refno in [{}] and aabb.d != none
            "#,
            zone_refnos
        )
    };

    println!("Query insts by zone sql: {}", &sql);

    let mut response = SUL_DB.query(sql).await?;
    let values: Vec<SurlValue> = response.take(0)?;
    let geom_insts: Vec<GeomInstQuery> = decode_values(values)?;

    Ok(geom_insts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RefnoEnum, init_test_surreal};

    #[tokio::test]
    async fn test_query_insts() -> anyhow::Result<()> {
        init_test_surreal().await;
        // Test case 1: Query single refno
        let refnos = vec!["17496/496442".into()];
        let result = query_insts(&refnos, false).await?;
        assert!(!result.is_empty(), "Should return at least one instance");
        dbg!(&result);

        // Verify returned instance has expected fields
        let first_inst = &result[0];
        // assert!(
        //     first_inst.world_aabb.is_some(),
        //     "World AABB should be present"
        // );
        // assert!(
        //     first_inst.world_trans.is_some(),
        //     "World transform should be present"
        // );
        // assert!(
        //     !first_inst.insts.is_empty(),
        //     "Should have geometry instances"
        // );

        assert!(
            first_inst.has_neg == true,
            "Should not have negative geometry"
        );

        // Test case 2: Query multiple refnos
        // let refnos = vec![RefnoEnum::Pe(24383_84088), RefnoEnum::Pe(24383_84089)];
        // let result = query_insts(&refnos).await?;
        // assert!(result.len() >= 2, "Should return multiple instances");

        // // Test case 3: Query non-existent refno
        // let refnos = vec![RefnoEnum::Pe(0)];
        // let result = query_insts(&refnos).await?;
        // assert!(
        //     result.is_empty(),
        //     "Should return empty for non-existent refno"
        // );

        Ok(())
    }

    #[tokio::test]
    async fn test_query_insts_by_zone() -> anyhow::Result<()> {
        init_test_surreal().await;

        // Test case: Query instances by zone
        let zone_refnos = vec!["24383_66457".into()];
        let result = query_insts_by_zone(&zone_refnos, false).await?;

        // Verify the results
        assert!(!result.is_empty(), "Should return instances for the zone");

        // Check the first instance has all required fields
        if let Some(first_inst) = result.first() {
            assert!(
                first_inst.refno.to_string().len() > 0,
                "Should have valid refno"
            );
            assert!(first_inst.insts.len() > 0, "Should have geometry instances");
        }

        Ok(())
    }
}

//=============================================================================
// inst_relate 数据保存相关函数
//=============================================================================

use crate::geometry::ShapeInstancesData;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;

/// 初始化数据库的 inst_relate 表的索引
pub async fn init_inst_relate_indices() -> anyhow::Result<()> {
    // 创建 zone_refno 字段的索引
    let create_index_sql = "
        DEFINE INDEX idx_inst_relate_zone_refno ON TABLE inst_relate COLUMNS zone_refno TYPE BTREE;
    ";
    let _ = SUL_DB.query(create_index_sql).await;
    Ok(())
}

/// 定义 dbnum_info_table 的更新事件
///
/// 当 pe 表有 CREATE/UPDATE/DELETE 事件时，自动更新 dbnum_info_table 的统计信息
#[cfg(feature = "surreal-save")]
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    let event_sql = r#"
    DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
            -- 获取当前记录的 dbnum
            LET $dbnum = $value.dbnum;
            LET $id = record::id($value.id);
            let $id_parts = string::split($id, "_");
            let $ref_0 = <int>array::at($id_parts, 0);
            let $ref_1 = <int>array::at($id_parts, 1);
            let $is_delete = $value.deleted and $event = "UPDATE";
            let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };
            -- 根据事件类型处理  type::record("dbnum_info_table", $ref_0)
            IF $event = "CREATE"   {
                UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
                    dbnum: $dbnum,
                    count: count?:0 + 1,
                    sesno: $max_sesno,
                    max_ref1: $ref_1,
                    updated_at: time::now()
                };
            } ELSE IF $event = "DELETE" OR $is_delete  {
                UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
                    count: count - 1,
                    sesno: $max_sesno,
                    max_ref1: $ref_1,
                    updated_at: time::now()
                }
                WHERE count > 0;
            }  ELSE IF $event = "UPDATE" {
                UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
                    sesno: $max_sesno,
                    updated_at: time::now()
                };
            };
        };
    "#;

    SUL_DB.query(event_sql).await?;
    Ok(())
}

/// 定义 dbnum_info_table 的更新事件 (非 surreal-save feature 时的空实现)
#[cfg(not(feature = "surreal-save"))]
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    Ok(())
}

///保存instance 数据到数据库（单线程版本）
pub async fn save_instance_data_single(
    inst_mgr: &ShapeInstancesData,
    replace_exist: bool,
) -> anyhow::Result<()> {
    use crate::gen_bytes_hash;
    use itertools::Itertools;

    let mut aabb_map: HashMap<u64, String> = HashMap::new();
    let mut transform_map: HashMap<u64, String> = HashMap::new();
    //标识单位矩阵
    transform_map.insert(0, serde_json::to_string(&Transform::IDENTITY).unwrap());
    let mut param_map = HashMap::new();
    let mut vec3_map: HashMap<u64, String> = HashMap::new();
    let test_refno = crate::get_db_option().get_test_refno();

    let chunk_size = 300;
    //把delete 提前，因为后面的插入都是异步的执行
    if replace_exist {
        let keys = inst_mgr.inst_info_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut delete_sql_vec = vec![];

            for &k in chunk {
                let v = inst_mgr.inst_info_map.get(k).unwrap();
                let delete_old_sql = format!(
                    r#"
                delete array::flatten(select value out->geo_relate.out from {0});
                delete array::flatten(select value out->geo_relate from {0});
                delete array::flatten(select value out from {0});
                delete {0};"#,
                    v.refno.to_inst_relate_key()
                );
                delete_sql_vec.push(delete_old_sql);
            }
            //如果需要删除之前的，先执行
            if !delete_sql_vec.is_empty() {
                let sql = delete_sql_vec.join("");
                // dbg!(&sql);
                SUL_DB.query(sql).await.unwrap();
            }
        }
        // return Ok(());
    }

    let keys = inst_mgr.inst_geos_map.keys().collect::<Vec<_>>();
    // let mut insert_handles = FuturesUnordered::new();
    let mut inst_geo_vec = vec![];
    let mut geo_relate_vec = vec![];

    // dbg!(&keys);
    for k in keys {
        let v = inst_mgr.inst_geos_map.get(k).unwrap();
        for inst in &v.insts {
            if inst.transform.translation.is_nan()
                || inst.transform.rotation.is_nan()
                || inst.transform.scale.is_nan()
            {
                dbg!(&inst);
                continue;
            }
            let transform_hash = crate::gen_bytes_hash(&inst.transform);
            if !transform_map.contains_key(&transform_hash) {
                transform_map.insert(
                    transform_hash,
                    serde_json::to_string(&inst.transform).unwrap(),
                );
            }
            let param_hash = crate::gen_bytes_hash(&inst.geo_param);
            if !param_map.contains_key(&param_hash) {
                param_map.insert(param_hash, serde_json::to_string(&inst.geo_param).unwrap());
            }
            let key_pts = inst.geo_param.key_points();
            let mut pt_hashes = vec![];
            for k in key_pts {
                let pts_hash = k.gen_hash();
                pt_hashes.push(format!("vec3:⟨{}⟩", pts_hash));
                if !vec3_map.contains_key(&pts_hash) {
                    vec3_map.insert(pts_hash, serde_json::to_string(&k).unwrap());
                }
            }
            //还需要加入geo_param的指向，param 是否填原始参数？ param=param:{}
            //使用cata_key -> inst_geos
            let cat_negs_str = if !inst.cata_neg_refnos.is_empty() {
                format!(
                    ", cata_neg: [{}]",
                    inst.cata_neg_refnos.iter().map(|x| x.to_pe_key()).join(",")
                )
            } else {
                "".to_string()
            };
            //如果是replace, 直接这里需要先删除之前的sql语句
            let mut relate_json = format!(
                r#"in: inst_info:⟨{0}⟩, out: inst_geo:⟨{1}⟩, trans: trans:⟨{2}⟩, geom_refno: pe:{3}, pts: [{4}], geo_type: '{5}', visible: {6} {7}"#,
                v.id(),
                inst.geo_hash,
                transform_hash,
                inst.refno,
                pt_hashes.join(","),
                inst.geo_type.to_string(),
                inst.visible,
                cat_negs_str
            );
            //将 string 转成一个 hash id
            let id = crate::gen_bytes_hash(&relate_json);
            let final_json = format!("{{ {relate_json}, id: '{id}' }}");
            // dbg!(&relate_sql);
            // println!("geo relate json: {}", &final_json);
            geo_relate_vec.push(final_json);
            //保存 unit shape 的几何参数
            inst_geo_vec.push(inst.gen_unit_geo_sur_json());
            // EXIST_MESH_GEOS.insert(inst.geo_hash);
        }
    }

    if !inst_geo_vec.is_empty() {
        for chunk in inst_geo_vec.chunks(chunk_size) {
            let sql_string = format!(
                "insert ignore into {} [{}];",
                stringify!(inst_geo),
                chunk.join(",")
            );
            // dbg!(&sql_string);
            // let handle = tokio::spawn(async move {
            SUL_DB.query(sql_string).await.unwrap();
            // });
            // insert_handles.push(handle);
        }
    }
    if !geo_relate_vec.is_empty() {
        // let handle = tokio::spawn(async move {
        for chunk in geo_relate_vec.chunks(chunk_size) {
            let sql = format!("INSERT RELATION INTO geo_relate [{}];", chunk.join(","));
            //
            // println!("geo relate sql: {}", &sql);
            let mut response = SUL_DB.query(sql).await.unwrap();
            // let mut error = response.take_errors();
            // if !error.is_empty() {
            //     dbg!(&error);
            // }
        }
        // });
        // insert_handles.push(handle);
    }

    //保存tubi的数据
    let keys = inst_mgr.inst_tubi_map.keys().collect::<Vec<_>>();
    for chunk in keys.chunks(chunk_size) {
        for &k in chunk {
            let v = inst_mgr.inst_tubi_map.get(k).unwrap();
            //更新aabb 和 transform，保存relate已经在别的地方加了，这里后面需要重构
            let aabb = v.aabb.unwrap();
            let aabb_hash = crate::gen_bytes_hash(&aabb);
            let transform_hash = crate::gen_bytes_hash(&v.world_transform);
            if !aabb_map.contains_key(&aabb_hash) {
                aabb_map.insert(aabb_hash, serde_json::to_string(&aabb).unwrap());
            }
            if !transform_map.contains_key(&transform_hash) {
                transform_map.insert(
                    transform_hash,
                    serde_json::to_string(&v.world_transform).unwrap(),
                );
            }
        }
    }

    let keys = inst_mgr.inst_info_map.keys().collect::<Vec<_>>();
    if !inst_mgr.neg_relate_map.is_empty() {
        let mut neg_relate_vec = vec![];
        // dbg!(&inst_mgr.neg_relate_map);
        for (k, refnos) in &inst_mgr.neg_relate_map {
            //这里需要order
            for (indx, r) in refnos.into_iter().enumerate() {
                neg_relate_vec.push(format!(
                    "{{ in: {}, id: [{}, {indx}], out: {} }}",
                    r.to_pe_key(),
                    r.to_string(),
                    k.to_pe_key(),
                ));
            }
        }
        if !neg_relate_vec.is_empty() {
            for chunk in neg_relate_vec.chunks(chunk_size) {
                let neg_relate_sql =
                    format!("INSERT RELATION INTO neg_relate [{}];", chunk.join(","));
                SUL_DB.query(neg_relate_sql).await.unwrap();
            }
        }
    }

    // dbg!(&inst_mgr.ngmr_neg_relate_map);
    if !inst_mgr.ngmr_neg_relate_map.is_empty() {
        let mut ngmr_relate_vec = vec![];
        for (k, refnos) in &inst_mgr.ngmr_neg_relate_map {
            let kpe = k.to_pe_key();
            for (ele_refno, ngmr_geom_refno) in refnos {
                let ele_pe = ele_refno.to_pe_key();
                let ngmr_pe = ngmr_geom_refno.to_pe_key();
                ngmr_relate_vec.push(format!(
                    "{{ in: {0}, id: [{0}, {1}, {2}], out: {1}, ngmr: {2}}}",
                    ele_pe, kpe, ngmr_pe
                ));
            }
        }
        if !ngmr_relate_vec.is_empty() {
            for chunk in ngmr_relate_vec.chunks(chunk_size) {
                let ngmr_relate_sql =
                    format!("INSERT RELATION INTO ngmr_relate [{}];", chunk.join(","));
                SUL_DB.query(ngmr_relate_sql).await.unwrap();
            }
        }
    }

    // dbg!(&inst_mgr.ngmr_relate_map);
    // for chunk in keys.chunks(chunk_size)
    {
        let mut inst_info_vec = vec![];
        let mut inst_relate_vec = vec![];
        for k in keys.clone() {
            let v = inst_mgr.inst_info_map.get(k).unwrap();
            if v.world_transform.translation.is_nan()
                || v.world_transform.rotation.is_nan()
                || v.world_transform.scale.is_nan()
            {
                continue;
            }
            inst_info_vec.push(v.gen_sur_json(&mut vec3_map));

            let transform_hash = crate::gen_bytes_hash(&v.world_transform);
            if !transform_map.contains_key(&transform_hash) {
                transform_map.insert(
                    transform_hash,
                    serde_json::to_string(&v.world_transform).unwrap(),
                );
            }

            let relate_sql = format!(
                "{{id: {},  in: {}, out: inst_info:⟨{}⟩, world_trans: trans:⟨{}⟩, generic: '{}', has_cata_neg: {}, solid: {}}}",
                k.to_inst_relate_key(),
                k.to_pe_key(),
                v.id_str(),
                transform_hash,
                v.generic_type.to_string(),
                v.has_cata_neg,
                v.is_solid,
                // v.dt.and_utc().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
            );
            if let Some(t_refno) = test_refno {
                if *k == t_refno.into() {
                    dbg!(v);
                    println!("inst relate sql: {}", &relate_sql);
                }
            }
            inst_relate_vec.push(relate_sql);
        }

        if !inst_info_vec.is_empty() {
            for chunk in inst_info_vec.chunks(chunk_size) {
                let sql_string = format!(
                    "insert ignore into {} [{}];",
                    stringify!(inst_info),
                    chunk.join(",")
                );
                SUL_DB.query(sql_string).await.unwrap();
            }
        }
        //inst relate 放到最后保存, 因为是被监控的
        if !inst_relate_vec.is_empty() {
            for chunk in inst_relate_vec.chunks(chunk_size) {
                let inst_relate_sql =
                    format!("INSERT RELATION INTO inst_relate [{}];", chunk.join(","));
                // println!("inst relate sql: {}", &inst_relate_sql);
                SUL_DB.query(inst_relate_sql).await.unwrap();
            }

            // 使用SQL函数更新zone_refno
            let update_zone_sql = "
                LET $records = SELECT * FROM inst_relate WHERE zone_refno = NONE;
                FOR $record IN $records {
                    LET $zone = fn::find_ancestor_type($record.in, 'ZONE');
                    IF $zone != NONE {
                        UPDATE $record SET zone_refno = $zone[0].refno;
                    }
                };
            ";
            SUL_DB.query(update_zone_sql).await.unwrap();

            for chunk in keys.to_vec().chunks(chunk_size) {
                let mut update_date_sql = String::new();
                for &k in chunk {
                    update_date_sql.push_str(&format!(
                        "update inst_relate:{k} set dt=fn::ses_date(pe:{k});"
                    ));
                }
                SUL_DB.query(update_date_sql).await.unwrap();
            }
        }
    }

    //保存aabb
    if !aabb_map.is_empty() {
        let keys = aabb_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut jsons = vec![];
            for &&k in chunk {
                let v = aabb_map.get(&k).unwrap();
                let json = format!("{{'id':aabb:⟨{}⟩, 'd':{}}}", k, v);
                jsons.push(json);
            }
            let sql = format!("INSERT IGNORE INTO aabb [{}];", jsons.join(","));
            SUL_DB.query(sql).await.unwrap();
        }
    }
    //保存transform
    if !transform_map.is_empty() {
        let keys = transform_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut sql_string = "".to_string();
            for &&k in chunk {
                let v = transform_map.get(&k).unwrap();
                let json = format!(
                    "INSERT IGNORE INTO trans {{'id':trans:⟨{}⟩, 'd':{}}};",
                    k, v
                );
                sql_string.push_str(&json);
            }
            SUL_DB.query(sql_string).await.unwrap();
        }
    }

    //保存vec3
    if !vec3_map.is_empty() {
        let keys = vec3_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut sql_string = "".to_string();
            for &&k in chunk {
                let v = vec3_map.get(&k).unwrap();
                let json = format!("INSERT IGNORE INTO vec3 {{'id':vec3:⟨{}⟩, 'd':{}}};", k, v);
                sql_string.push_str(&json);
            }
            SUL_DB.query(sql_string).await.unwrap();
        }
    }

    //保存param
    if !param_map.is_empty() {
        let keys = param_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut sql_string = "".to_string();
            for &&k in chunk {
                let v = param_map.get(&k).unwrap();
                let json = format!(
                    "INSERT IGNORE INTO param {{'id':param:⟨{}⟩, 'd':{}}};",
                    k, v
                );
                sql_string.push_str(&json);
            }
            SUL_DB.query(sql_string).await.unwrap();
        }
    }

    Ok(())
}

///保存instance 数据到数据库（并发优化版本）
pub async fn save_instance_data(
    inst_mgr: &ShapeInstancesData,
    replace_exist: bool,
) -> anyhow::Result<()> {
    use crate::gen_bytes_hash;
    use itertools::Itertools;

    // ========== 调试信息开始 ==========
    println!("\n=== save_instance_data 被调用 ===");
    println!("inst_info_map.len() = {}", inst_mgr.inst_info_map.len());
    println!("inst_geos_map.len() = {}", inst_mgr.inst_geos_map.len());
    println!("inst_tubi_map.len() = {}", inst_mgr.inst_tubi_map.len());
    println!("neg_relate_map.len() = {}", inst_mgr.neg_relate_map.len());
    println!(
        "ngmr_neg_relate_map.len() = {}",
        inst_mgr.ngmr_neg_relate_map.len()
    );
    println!("replace_exist = {}", replace_exist);
    // ========== 调试信息结束 ==========

    let mut aabb_map: HashMap<u64, String> = HashMap::new();
    let mut transform_map: HashMap<u64, String> = HashMap::new();
    //标识单位矩阵
    transform_map.insert(0, serde_json::to_string(&Transform::IDENTITY).unwrap());
    let mut param_map = HashMap::new();
    let mut vec3_map: HashMap<u64, String> = HashMap::new();
    let test_refno = crate::get_db_option().get_test_refno();

    let chunk_size = 300;

    // 创建一个任务集合来管理并发操作
    let mut db_futures = FuturesUnordered::new();

    //把delete 提前，因为后面的插入都是异步的执行
    if replace_exist {
        let keys = inst_mgr.inst_info_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut delete_sql_vec = vec![];

            for &k in chunk {
                let v = inst_mgr.inst_info_map.get(k).unwrap();
                let delete_old_sql = format!(
                    r#"
                delete array::flatten(select value out->geo_relate.out from {0});
                delete array::flatten(select value out->geo_relate from {0});
                delete array::flatten(select value out from {0});
                delete {0};"#,
                    v.refno.to_inst_relate_key()
                );
                delete_sql_vec.push(delete_old_sql);
            }
            //如果需要删除之前的，先执行
            if !delete_sql_vec.is_empty() {
                let sql = delete_sql_vec.join("");
                // 这里需要同步等待删除操作完成
                SUL_DB.query(sql).await.unwrap();
            }
        }
    }

    let keys = inst_mgr.inst_geos_map.keys().collect::<Vec<_>>();
    let mut inst_geo_vec = vec![];
    let mut geo_relate_vec = vec![];

    // 准备inst_geo和geo_relate数据
    for k in keys {
        let v = inst_mgr.inst_geos_map.get(k).unwrap();
        for inst in &v.insts {
            if inst.transform.translation.is_nan()
                || inst.transform.rotation.is_nan()
                || inst.transform.scale.is_nan()
            {
                dbg!(&inst);
                continue;
            }
            let transform_hash = crate::gen_bytes_hash(&inst.transform);
            if !transform_map.contains_key(&transform_hash) {
                transform_map.insert(
                    transform_hash,
                    serde_json::to_string(&inst.transform).unwrap(),
                );
            }
            let param_hash = crate::gen_bytes_hash(&inst.geo_param);
            if !param_map.contains_key(&param_hash) {
                param_map.insert(param_hash, serde_json::to_string(&inst.geo_param).unwrap());
            }
            let key_pts = inst.geo_param.key_points();
            let mut pt_hashes = vec![];
            for k in key_pts {
                let pts_hash = k.gen_hash();
                pt_hashes.push(format!("vec3:⟨{}⟩", pts_hash));
                if !vec3_map.contains_key(&pts_hash) {
                    vec3_map.insert(pts_hash, serde_json::to_string(&k).unwrap());
                }
            }
            //还需要加入geo_param的指向，param 是否填原始参数？ param=param:{}
            //使用cata_key -> inst_geos
            let cat_negs_str = if !inst.cata_neg_refnos.is_empty() {
                format!(
                    ", cata_neg: [{}]",
                    inst.cata_neg_refnos.iter().map(|x| x.to_pe_key()).join(",")
                )
            } else {
                "".to_string()
            };
            //如果是replace, 直接这里需要先删除之前的sql语句
            let mut relate_json = format!(
                r#"in: inst_info:⟨{0}⟩, out: inst_geo:⟨{1}⟩, trans: trans:⟨{2}⟩, geom_refno: pe:{3}, pts: [{4}], geo_type: '{5}', visible: {6} {7}"#,
                v.id(),
                inst.geo_hash,
                transform_hash,
                inst.refno,
                pt_hashes.join(","),
                inst.geo_type.to_string(),
                inst.visible,
                cat_negs_str
            );
            //将 string 转成一个 hash id
            let id = crate::gen_bytes_hash(&relate_json);
            let final_json = format!("{{ {relate_json}, id: '{id}' }}");
            geo_relate_vec.push(final_json);
            //保存 unit shape 的几何参数
            inst_geo_vec.push(inst.gen_unit_geo_sur_json());
        }
    }

    // 并发保存inst_geo数据
    if !inst_geo_vec.is_empty() {
        for chunk in inst_geo_vec.chunks(chunk_size) {
            let sql_string = format!(
                "insert ignore into {} [{}];",
                stringify!(inst_geo),
                chunk.join(",")
            );
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql_string).await });
            db_futures.push(future);
        }
    }

    // 并发保存geo_relate数据
    if !geo_relate_vec.is_empty() {
        for chunk in geo_relate_vec.chunks(chunk_size) {
            let sql = format!("INSERT RELATION INTO geo_relate [{}];", chunk.join(","));
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql).await });
            db_futures.push(future);
        }
    }

    // 处理tubi数据
    let keys = inst_mgr.inst_tubi_map.keys().collect::<Vec<_>>();
    for chunk in keys.chunks(chunk_size) {
        for &k in chunk {
            let v = inst_mgr.inst_tubi_map.get(k).unwrap();
            //更新aabb 和 transform，保存relate已经在别的地方加了，这里后面需要重构
            let aabb = v.aabb.unwrap();
            let aabb_hash = crate::gen_bytes_hash(&aabb);
            let transform_hash = crate::gen_bytes_hash(&v.world_transform);
            if !aabb_map.contains_key(&aabb_hash) {
                aabb_map.insert(aabb_hash, serde_json::to_string(&aabb).unwrap());
            }
            if !transform_map.contains_key(&transform_hash) {
                transform_map.insert(
                    transform_hash,
                    serde_json::to_string(&v.world_transform).unwrap(),
                );
            }
        }
    }

    // 处理负关系数据并并发保存
    if !inst_mgr.neg_relate_map.is_empty() {
        let mut neg_relate_vec = vec![];
        for (k, refnos) in &inst_mgr.neg_relate_map {
            for (indx, r) in refnos.into_iter().enumerate() {
                neg_relate_vec.push(format!(
                    "{{ in: {}, id: [{}, {indx}], out: {} }}",
                    r.to_pe_key(),
                    r.to_string(),
                    k.to_pe_key(),
                ));
            }
        }
        if !neg_relate_vec.is_empty() {
            for chunk in neg_relate_vec.chunks(chunk_size) {
                let neg_relate_sql =
                    format!("INSERT RELATION INTO neg_relate [{}];", chunk.join(","));
                let db = SUL_DB.clone();
                let future = tokio::spawn(async move { db.query(neg_relate_sql).await });
                db_futures.push(future);
            }
        }
    }

    // 处理ngmr负关系数据并并发保存
    if !inst_mgr.ngmr_neg_relate_map.is_empty() {
        let mut ngmr_relate_vec = vec![];
        for (k, refnos) in &inst_mgr.ngmr_neg_relate_map {
            let kpe = k.to_pe_key();
            for (ele_refno, ngmr_geom_refno) in refnos {
                let ele_pe = ele_refno.to_pe_key();
                let ngmr_pe = ngmr_geom_refno.to_pe_key();
                ngmr_relate_vec.push(format!(
                    "{{ in: {0}, id: [{0}, {1}, {2}], out: {1}, ngmr: {2}}}",
                    ele_pe, kpe, ngmr_pe
                ));
            }
        }
        if !ngmr_relate_vec.is_empty() {
            for chunk in ngmr_relate_vec.chunks(chunk_size) {
                let ngmr_relate_sql =
                    format!("INSERT RELATION INTO ngmr_relate [{}];", chunk.join(","));
                let db = SUL_DB.clone();
                let future = tokio::spawn(async move { db.query(ngmr_relate_sql).await });
                db_futures.push(future);
            }
        }
    }

    // 处理inst_info数据
    let keys = inst_mgr.inst_info_map.keys().collect::<Vec<_>>();
    let mut inst_info_vec = vec![];
    let mut inst_relate_vec = vec![];

    for k in keys.clone() {
        let v = inst_mgr.inst_info_map.get(k).unwrap();
        if v.world_transform.translation.is_nan()
            || v.world_transform.rotation.is_nan()
            || v.world_transform.scale.is_nan()
        {
            continue;
        }
        inst_info_vec.push(v.gen_sur_json(&mut vec3_map));

        let transform_hash = crate::gen_bytes_hash(&v.world_transform);
        if !transform_map.contains_key(&transform_hash) {
            transform_map.insert(
                transform_hash,
                serde_json::to_string(&v.world_transform).unwrap(),
            );
        }

        let relate_sql = format!(
            "{{id: {0},  in: {1}, out: inst_info:⟨{2}⟩, world_trans: trans:⟨{3}⟩, generic: '{4}', zone_refno: fn::find_ancestor_type({1}, 'ZONE'), dt: fn::ses_date({1}), has_cata_neg: {5}, solid: {6}}}",
            k.to_inst_relate_key(),
            k.to_pe_key(),
            v.id_str(),
            transform_hash,
            v.generic_type.to_string(),
            v.has_cata_neg,
            v.is_solid,
        );
        if let Some(t_refno) = test_refno {
            if *k == t_refno.into() {
                dbg!(v);
                println!("inst relate sql: {}", &relate_sql);
            }
        }
        inst_relate_vec.push(relate_sql);
    }

    if !inst_relate_vec.is_empty() {
        println!("准备插入 inst_relate: {} 条记录", inst_relate_vec.len());
        for chunk in inst_relate_vec.chunks(chunk_size) {
            let inst_relate_sql =
                format!("INSERT RELATION INTO inst_relate [{}];", chunk.join(","));
            println!("inst_relate SQL chunk size: {}", chunk.len());
            // 打印第一条SQL的前200个字符用于调试
            if inst_relate_vec.len() > 0 {
                let sample = &inst_relate_sql[..200.min(inst_relate_sql.len())];
                println!("inst_relate SQL sample: {}...", sample);
            }
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(inst_relate_sql).await });
            db_futures.push(future);
        }
    } else {
        println!("⚠️ inst_relate_vec 为空，没有数据需要插入！");
    }

    // 并发保存inst_info数据
    if !inst_info_vec.is_empty() {
        for chunk in inst_info_vec.chunks(chunk_size) {
            let sql_string = format!(
                "insert ignore into {} [{}];",
                stringify!(inst_info),
                chunk.join(",")
            );
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql_string).await });
            db_futures.push(future);
        }
    }

    // 并发保存aabb数据
    if !aabb_map.is_empty() {
        let keys = aabb_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut jsons = vec![];
            for &&k in chunk {
                let v = aabb_map.get(&k).unwrap();
                let json = format!("{{'id':aabb:⟨{}⟩, 'd':{}}}", k, v);
                jsons.push(json);
            }
            let sql = format!("INSERT IGNORE INTO aabb [{}];", jsons.join(","));
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql).await });
            db_futures.push(future);
        }
    }

    // 并发保存transform数据（优化批量插入语法）
    if !transform_map.is_empty() {
        let keys = transform_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut jsons = vec![];
            for &&k in chunk {
                let v = transform_map.get(&k).unwrap();
                jsons.push(format!("{{'id':trans:⟨{}⟩, 'd':{}}}", k, v));
            }
            let sql = format!("INSERT IGNORE INTO trans [{}];", jsons.join(","));
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql).await });
            db_futures.push(future);
        }
    }

    // 并发保存vec3数据（优化批量插入语法）
    if !vec3_map.is_empty() {
        let keys = vec3_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut jsons = vec![];
            for &&k in chunk {
                let v = vec3_map.get(&k).unwrap();
                jsons.push(format!("{{'id':vec3:⟨{}⟩, 'd':{}}}", k, v));
            }
            let sql = format!("INSERT IGNORE INTO vec3 [{}];", jsons.join(","));
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql).await });
            db_futures.push(future);
        }
    }

    // 并发保存param数据
    if !param_map.is_empty() {
        let keys = param_map.keys().collect::<Vec<_>>();
        for chunk in keys.chunks(chunk_size) {
            let mut jsons = vec![];
            for &&k in chunk {
                let v = param_map.get(&k).unwrap();
                jsons.push(format!("{{'id':param:⟨{}⟩, 'd':{}}}", k, v));
            }
            let sql = format!("INSERT IGNORE INTO param [{}];", jsons.join(","));
            let db = SUL_DB.clone();
            let future = tokio::spawn(async move { db.query(sql).await });
            db_futures.push(future);
        }
    }

    // 等待所有并发任务完成
    println!("等待 {} 个数据库任务完成...", db_futures.len());
    let mut completed = 0;
    let mut errors = 0;
    while let Some(result) = db_futures.next().await {
        completed += 1;
        if let Err(e) = result {
            errors += 1;
            eprintln!("❌ Task join error #{}: {:?}", errors, e);
        } else if let Ok(query_result) = result {
            if let Err(db_err) = query_result {
                errors += 1;
                eprintln!("❌ Database query error #{}: {:?}", errors, db_err);
            }
        }
    }
    println!("✅ 完成 {} 个任务，{} 个错误", completed, errors);

    // 注意：zone_refno 和 dt 已经在 inst_relate SQL 中直接设置了
    // 使用 SQL 函数 fn::find_ancestor_type 和 fn::ses_date
    // 所以不需要额外的更新操作

    println!("=== save_instance_data 完成 ===\n");
    Ok(())
}
