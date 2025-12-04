use crate::basic::aabb::ParryAabb;
use crate::pdms_types::PdmsGenericType;
use crate::rs_surreal::geometry_query::PlantTransform;
use crate::shape::pdms_shape::RsVec3;
use crate::types::PlantAabb;
use crate::{RefU64, RefnoEnum, SUL_DB, SurlValue, SurrealQueryExt, get_inst_relate_keys};
use anyhow::Context;
use bevy_transform::components::Transform;
use chrono::{DateTime, Local, NaiveDateTime};
use glam::{DVec3, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use serde_with::serde_as;
use surrealdb::types as surrealdb_types;
use surrealdb::types::{Kind, SurrealValue, Value};

/// 完整的 Ptset 点数据，包含位置和方向信息
#[derive(Serialize, Deserialize, Debug, Clone, Default, SurrealValue)]
pub struct FullPtsetPoint {
    /// 点位置
    pub pt: RsVec3,
    /// 主方向（连接方向）
    #[serde(default)]
    pub dir: Option<RsVec3>,
    /// 参考方向
    #[serde(default)]
    pub ref_dir: Option<RsVec3>,
    /// 点编号
    #[serde(default)]
    pub number: i32,
    /// 方向标志
    #[serde(default)]
    pub dir_flag: f32,
    /// 口径
    #[serde(default)]
    pub pbore: f32,
    /// 连接类型
    #[serde(default)]
    pub pconnect: String,
}

/// 初始化数据库的 inst_relate 表的索引
pub async fn init_inst_relate_indices() -> anyhow::Result<()> {
    // 创建 zone_refno 字段的索引
    let create_index_sql = "
        DEFINE INDEX idx_inst_relate_zone_refno ON TABLE inst_relate COLUMNS zone_refno TYPE BTREE;
    ";
    let _ = SUL_DB.query_response(create_index_sql).await;
    Ok(())
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct TubiInstQuery {
    pub refno: RefnoEnum,
    pub leave: RefnoEnum,
    pub old_refno: Option<RefnoEnum>,
    pub generic: Option<String>,
    pub world_aabb: PlantAabb,
    pub world_trans: PlantTransform,
    pub geo_hash: String,
    pub date: Option<surrealdb::types::Datetime>,
}

/// 将 SurrealDB 的原始值向量解码为目标类型列表
///
/// # 参数
///
/// * `values` - 从查询结果中获取的 `SurlValue` 向量
///
/// # 返回值
///
/// 返回解码后的目标类型向量，若解码失败则返回错误
fn decode_values<T: DeserializeOwned>(values: Vec<SurlValue>) -> anyhow::Result<Vec<T>> {
    values
        .into_iter()
        .map(|value| {
            let json = value.into_json_value();
            serde_json::from_value(json).context("failed to deserialize Surreal value")
        })
        .collect()
}

/// 根据分支构件编号批量查询 Tubi 实例数据
///
/// # 参数
///
/// * `bran_refnos` - 需要查询的分支构件编号切片
///
/// # 返回值
///
/// 返回符合条件的 `TubiInstQuery` 列表
///
/// # 注意
///
/// `tubi_relate` 表的 ID 格式是 `[pe:⟨第一个子元素refno⟩, index]`，
/// 而不是 `[pe:⟨BRAN_refno⟩, index]`。因此需要先查询 BRAN 的第一个子元素，
/// 然后用它来查询 `tubi_relate`。
pub async fn query_tubi_insts_by_brans(
    bran_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<TubiInstQuery>> {
    if bran_refnos.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_results = Vec::new();
    for bran_refno in bran_refnos {
        let bran_id = bran_refno.to_e3d_id();
        
        // 通过 owner 字段查询 tubi_relate
        // tubi_relate 的 id[0] 是 pe 记录，其 owner 字段指向 BRAN
        // 注意：使用 pe_key 格式直接比较 record，避免字符串转换的性能开销
        let pe_key = bran_refno.to_pe_key();
        let sql = format!(
            r#"
            SELECT
                id[0] as refno,
                in as leave,
                id[0].old_pe as old_refno,
                id[0].owner.noun as generic,
                aabb.d as world_aabb,
                world_trans.d as world_trans,
                record::id(geo) as geo_hash,
                id[0].dt as date
            FROM tubi_relate
            WHERE id[0].owner = {} AND aabb.d != NONE
            "#,
            pe_key
        );
        let mut results: Vec<TubiInstQuery> = SUL_DB.query_take(&sql, 0).await?;
        
        all_results.append(&mut results);
    }
    Ok(all_results)
}

/// 根据流程构件编号批量查询 Tubi 实例数据
///
/// # 参数
///
/// * `refnos` - 需要查询的流程构件编号切片
///
/// # 返回值
///
/// 返回符合条件的 `TubiInstQuery` 列表
pub async fn query_tubi_insts_by_flow(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<TubiInstQuery>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_results = Vec::new();
    for refno in refnos {
        let pe_key = refno.to_pe_key();
        let sql = format!(
            r#"
            SELECT
                id[0] as refno,
                in as leave,
                id[0].old_pe as old_refno,
                id[0].owner.noun as generic,
                aabb.d as world_aabb,
                world_trans.d as world_trans,
                record::id(geo) as geo_hash,
                id[0].dt as date
            FROM tubi_relate
            WHERE (in = {} OR out = {}) AND aabb.d != NONE
            "#,
            pe_key,
            pe_key
        );

        let mut results: Vec<TubiInstQuery> = SUL_DB.query_take(&sql, 0).await?;
        all_results.append(&mut results);
    }

    Ok(all_results)
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, SurrealValue)]
pub struct ModelHashInst {
    pub geo_hash: String,
    #[serde(default)]
    pub transform: PlantTransform,
    #[serde(default)]
    pub is_tubi: bool,
    /// 是否为单位 mesh：true=通过 transform 缩放，false=通过 mesh 顶点缩放
    /// SQL 查询需使用 `?? false` 处理 NULL 值
    #[serde(default)]
    pub unit_flag: bool,
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
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct GeomInstQuery {
    /// 构件编号，别名为id
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 历史构件编号
    pub old_refno: Option<RefnoEnum>,
    /// 所属构件编号
    pub owner: RefnoEnum,
    /// 世界坐标系下的包围盒
    pub world_aabb: PlantAabb,
    /// 世界坐标系下的变换矩阵
    pub world_trans: PlantTransform,
    /// 几何实例列表
    pub insts: Vec<ModelHashInst>,
    /// 是否包含负实体
    pub has_neg: bool,
    /// 构件类型
    pub generic: String,
    /// 点集数据
    pub pts: Option<Vec<RsVec3>>,
    /// 时间戳
    pub date: Option<surrealdb::types::Datetime>,
}

/// 几何点集查询结构体
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct GeomPtsQuery {
    /// 构件编号，别名为id
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 世界坐标系下的变换矩阵
    pub world_trans: PlantTransform,
    /// 世界坐标系下的包围盒
    pub world_aabb: PlantAabb,
    /// 点集组，每组包含一个变换矩阵和可选的点集数据
    pub pts_group: Vec<(PlantTransform, Option<Vec<RsVec3>>)>,
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
    query_insts_with_batch(refnos, enable_holes, None).await
}

pub async fn query_insts_with_batch(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
    batch_size: Option<usize>,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let batch = batch_size.unwrap_or(50).max(1);
    let mut results = Vec::new();
    for chunk in refnos.chunks(batch) {
        let inst_keys = get_inst_relate_keys(chunk);

        // pts 从 inst_geo.pts 获取（vec3 引用数组），解引用获取实际坐标 .d
        // 使用子查询从第一个有效的 geo_relate.out.pts 获取
        let sql = if enable_holes {
            format!(
                r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans,
                (select value out.pts.*.d from out->geo_relate where visible && out.meshed && out.pts != none limit 1)[0] as pts,
                if booled_id != none {{ [{{ "geo_hash": booled_id, "transform": world_trans.d, "is_tubi": false, "unit_flag": false }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash, false as is_tubi, out.unit_flag ?? (record::id(out) INSIDE ['1', '2', '3']) as unit_flag from out->geo_relate where visible && out.meshed && trans.d != none)  }} as insts,
                booled_id != none as has_neg,
                <datetime>dt as date
            from {inst_keys} where aabb.d != none && world_trans.d != none
        "#
            )
        } else {
            format!(
                r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans,
                (select value out.pts.*.d from out->geo_relate where visible && out.meshed && out.pts != none limit 1)[0] as pts,
                (select trans.d as transform, record::id(out) as geo_hash, false as is_tubi, out.unit_flag ?? (record::id(out) INSIDE ['1', '2', '3']) as unit_flag from out->geo_relate where visible && out.meshed && trans.d != none) as insts,
                booled_id != none as has_neg,
                <datetime>dt as date
            from {inst_keys} where aabb.d != none && world_trans.d != none "#
            )
        };

        let mut chunk_result: Vec<GeomInstQuery> = SUL_DB.query_take(&sql, 0).await?;
        results.append(&mut chunk_result);
    }

    Ok(results)
}

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
//     let mut response = SUL_DB.query_response(sql).await?;
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
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");

    // 临时方案：使用 in.dt 替代 fn::ses_date(in.id) 以避免 "Expected any, got record" 错误
    // TODO: 确认 in.dt 字段是否可用，或者使用其他方案
    let sql = if enable_holes {
        format!(
            r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset[*].pt as pts,
                if booled_id != none {{ [{{ "geo_hash": booled_id }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none)  }} as insts,
                booled_id != none as has_neg,
                in.dt as date
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
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset[*].pt as pts,
                (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none) as insts,
                booled_id != none as has_neg,
                in.dt as date
            from inst_relate where zone_refno in [{}] and aabb.d != none
            "#,
            zone_refnos
        )
    };

    println!("Query insts by zone sql: {}", &sql);

    let mut response = SUL_DB.query_response(&sql).await?;
    let values: Vec<SurlValue> = response.take(0)?;
    let geom_insts: Vec<GeomInstQuery> = decode_values(values)?;

    Ok(geom_insts)
}

//=============================================================================
// inst_relate 数据保存相关函数
//=============================================================================

use crate::geometry::ShapeInstancesData;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;

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

    SUL_DB.query_response(event_sql).await?;
    Ok(())
}

/// 定义 dbnum_info_table 的更新事件 (非 surreal-save feature 时的空实现)
#[cfg(not(feature = "surreal-save"))]
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    Ok(())
}

/// 级联删除 inst_relate 及其关联的 geo_relate 和 inst_geo 数据
///
/// 当 replace_mesh 开启时，需要完全删除之前生成的数据，包括：
/// - inst_geo: 几何体节点
/// - geo_relate: 几何关系边
/// - inst_info: 实例信息节点
/// - inst_relate: 实例关系边
///
/// # 参数
/// * `refnos` - 需要删除的 refno 列表
/// * `chunk_size` - 分批处理的大小
///
/// # 删除顺序
/// 1. inst_geo (最外层)
/// 2. geo_relate (关系边)
/// 3. inst_info (信息节点)
/// 4. inst_relate (关系边)
pub async fn delete_inst_relate_cascade(
    refnos: &[RefnoEnum],
    chunk_size: usize,
) -> anyhow::Result<()> {
    for chunk in refnos.chunks(chunk_size) {
        let mut delete_sql_vec = vec![];

        let mut inst_ids = vec![];
        for &refno in chunk {
            inst_ids.push(refno.to_inst_relate_key());
            let delete_sql = format!(
                r#"
                    delete array::flatten(select value [out, id, in] from {}->inst_info->geo_relate);
                "#,
                refno.to_inst_relate_key()
            );
            delete_sql_vec.push(delete_sql);
        }

        if !delete_sql_vec.is_empty() {
            let mut sql = "BEGIN TRANSACTION;\n".to_string();
            sql.push_str(&delete_sql_vec.join(""));
            sql.push_str(&format!("delete {};", inst_ids.join(",")));
            sql.push_str("\nCOMMIT TRANSACTION;");
            // println!("Delete Sql is {}", &sql);
            SUL_DB
                .query(sql)
                .await
                .expect("delete model insts info failed");
        }
    }

    Ok(())
}

/// 删除所有模型生成相关的数据
///
/// 删除 inst_relate、inst_geo、inst_info、geo_relate 四个表中的所有数据
///
/// # 参数
/// * `chunk_size` - 分批处理的大小
pub async fn delete_all_model_data() -> anyhow::Result<()> {
    let tables = [
        "inst_relate",
        "inst_geo",
        "inst_info",
        "tubi_relate",
        "geo_relate",
        "neg_relate",
        "ngmr_relate",
    ];
    let mut sql = "BEGIN TRANSACTION;\n".to_string();

    for table in &tables {
        sql.push_str(&format!("delete {};\n", table));
    }

    sql.push_str("COMMIT TRANSACTION;");

    println!("Delete Sql is: \n {}", &sql);

    SUL_DB.query(sql).await.unwrap();
    Ok(())
}
