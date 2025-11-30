use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;
use crate::SurrealQueryExt;
/// Boolean operation query functions
///
/// This module provides query functions for boolean operations on geometry,
/// including catalog negative boolean groups, manifold operations, and OCC operations.
use crate::rs_surreal::query_structs::{
    CataNegGroup, GmGeoData, ManiGeoTransQuery, OccGeoTransQuery,
};
use crate::types::RefnoEnum;
use crate::{SUL_DB, get_inst_relate_keys};

/// Query catalog negative boolean groups
///
/// # Parameters
///
/// * `refnos` - Array of reference numbers
/// * `replace_exist` - Whether to replace existing boolean operation results
///
/// # Returns
///
/// Returns `Vec<CataNegGroup>` containing catalog negative boolean groups
///
/// # SQL Query
///
/// Selects instances with catalog negative geometry:
/// - Filters by `has_cata_neg` flag
/// - Optionally filters by `!bad_bool and !booled` if not replacing
/// - Returns refno, inst_info_id, and boolean_group (flattened geom_refno and cata_neg)
pub async fn query_cata_neg_boolean_groups(
    refnos: &[RefnoEnum],
    replace_exist: bool,
) -> anyhow::Result<Vec<CataNegGroup>> {
    let inst_keys = get_inst_relate_keys(refnos);

    let mut sql = format!(
        r#" select in as refno, (->inst_info)[0] as inst_info_id, (select value array::flatten([geom_refno, cata_neg])
            from ->inst_info->geo_relate where visible and !out.bad and cata_neg!=none) as boolean_group
            from {inst_keys} where in.id != none and (->inst_info)[0]!=none and has_cata_neg "#
    );

    if !replace_exist {
        sql.push_str("and !bad_bool and !booled");
    }

    SUL_DB.query_take(&sql, 0).await
}

/// Query geometry mesh data
///
/// # Parameters
///
/// * `refno` - Reference number
/// * `geom_refnos` - Array of geometry reference numbers to query
///
/// # Returns
///
/// Returns `Vec<GmGeoData>` containing geometry mesh data (ID, transform, param, aabb)
///
/// # SQL Query
///
/// Selects geometry data from inst_relate->inst_info->geo_relate:
/// - Filters by geom_refno in the provided list
/// - Requires aabb and param to be non-null
/// - Returns record ID, geom_refno, transform, param, and aabb_id
pub async fn query_geom_mesh_data(
    refno: RefnoEnum,
    geom_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<GmGeoData>> {
    let pes = geom_refnos
        .iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        r#"
        select out as id, geom_refno, trans.d as trans, out.param as param, out.aabb as aabb_id
        from {}->inst_relate->inst_info->geo_relate
        where !out.bad and geom_refno in [{}]  and out.aabb!=none and out.param!=none"#,
        refno.to_pe_key(),
        pes
    );

    SUL_DB.query_take(&sql, 0).await
}

/// Query manifold boolean operations
///
/// # Parameters
///
/// * `refno` - Reference number (正实体 refno)
///
/// # Returns
///
/// Returns `Vec<ManiGeoTransQuery>` containing manifold boolean operation data
///
/// # SQL Query
///
/// Selects instances with negative geometry relationships:
/// - Filters instances with neg_relate or ngmr_relate pointing to the given refno
/// - 关系方向：负实体 -[neg_relate/ngmr_relate]-> 正实体
/// - 使用反向查询 `(in<-neg_relate)[0]` 和 `(in<-ngmr_relate)[0]` 查找指向正实体的关系
/// - Returns refno, sesno, noun, world_trans, aabb
/// - Returns positive geometries (Compound/Pos) with transforms
/// - Returns negative geometries (Neg/CataCrossNeg) with transforms and aabb
pub async fn query_manifold_boolean_operations(
    refno: RefnoEnum,
) -> anyhow::Result<Vec<ManiGeoTransQuery>> {
    let sql = format!(
        r#"
        select
                in as refno,
                in.sesno as sesno,
                in.noun as noun,
                world_trans.d as wt,
                aabb.d as aabb,
                (select value [out, trans.d] from out->geo_relate where geo_type in ["Compound", "Pos"] and trans.d != NONE ) as ts,
                (select value [in, world_trans.d,
                    (select out as id, geo_type, para_type ?? "" as para_type, trans.d as trans, out.aabb.d as aabb
                    from array::flatten(out->geo_relate) where trans.d != NONE and ( geo_type=="Neg" or (geo_type=="CataCrossNeg"
                        and geom_refno in (select value ngmr from pe:{refno}<-ngmr_relate) ) ))]
                        from array::flatten([array::flatten(in<-neg_relate.in<-inst_relate), array::flatten(in<-ngmr_relate.in->inst_relate)]) where world_trans.d!=none
                ) as neg_ts
             from inst_relate:{refno} where in.id != none and !bad_bool and aabb.d != NONE
        "#
    );

println!("[query_manifold_boolean_operations] refno: {}", refno);
    println!("[query_manifold_boolean_operations] sql: {}", sql);

    let result = SUL_DB.query_take::<Vec<ManiGeoTransQuery>>(&sql, 0).await;
    match &result {
        Ok(data) => {
            println!(
                "[query_manifold_boolean_operations] 查询成功，返回 {} 条结果",
                data.len()
            );
            {
                for (i, item) in data.iter().take(3).enumerate() {
                    println!(
                        "  结果 {}: refno={}, ts.len()={}, neg_ts.len()={}",
                        i,
                        item.refno,
                        item.ts.len(),
                        item.neg_ts.len()
                    );
                }
            }
        }
        Err(e) => {
            println!("[query_manifold_boolean_operations] 查询失败: {:?}", e);
        }
    }
    result
}



/// Query simple catalog negative boolean operations
///
/// # Returns
///
/// Returns `Vec<CataNegGroup>` containing simple catalog negative boolean groups
///
/// # SQL Query
///
/// Similar to query_cata_neg_boolean_groups but without refno filtering.
/// Used for catalog-level boolean operations.
pub async fn query_simple_cata_negative_bool() -> anyhow::Result<Vec<CataNegGroup>> {
    let sql = r#"
        select in as refno, (->inst_info)[0] as inst_info_id,
        (select value array::flatten([geom_refno, cata_neg])
            from ->inst_info->geo_relate where visible and !out.bad and cata_neg!=none) as boolean_group
        from inst_relate where in.id != none and (->inst_info)[0]!=none and has_cata_neg and !bad_bool and !booled
    "#;

    SUL_DB.query_take(sql, 0).await
}

/// 批量查询指定范围内需要布尔运算的正实体
///
/// 规则：存在任一入边 neg_relate 或 ngmr_relate 即视为需要布尔计算的正实体。
/// - 当 `refnos` 非空：仅在给定集合内筛选
/// - 当 `refnos` 为空：在全库正实体（pe）范围内扫描
///
/// 返回：正实体的记录 id（RefnoEnum）
pub async fn query_manifold_boolean_targets_in(
    refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        // 全库扫描：遍历 pe 表，筛选存在入边的正实体
        let sql = r#"
            SELECT VALUE id FROM pe
            WHERE array::len(array::union(<-neg_relate.in, <-ngmr_relate.in)) > 0
        "#;
        return SUL_DB.query_take(sql, 0).await;
    }

    let refno_keys = refnos
        .iter()
        .map(|r| r.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        r#"
        SELECT VALUE id FROM [{refno_keys}]
        WHERE array::len(array::union(<-neg_relate.in, <-ngmr_relate.in)) > 0
        "#
    );

    SUL_DB.query_take(&sql, 0).await
}

/// 全库扫描（分页）查询需要布尔运算的正实体
///
/// 使用 START/LIMIT 对大数据集做分页，避免一次性拉取过多 id。
pub async fn query_manifold_boolean_targets_scan_paginated(
    start: usize,
    limit: usize,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!(
        r#"
        SELECT VALUE id FROM pe
        WHERE array::len(array::union(<-neg_relate.in, <-ngmr_relate.in)) > 0
        START {start}
        LIMIT {limit}
        "#
    );

    SUL_DB.query_take(&sql, 0).await
}


/// 简化版：根据已知正实体 refno 获取布尔运算所需数据（多条简单查询组装）
///
/// 思路：
/// - 先取正实体的基础信息 + 正向几何 ts（单条 SELECT 带一个简单子查询）
/// - 再分别查询 Neg 与 Ngmr 两类负载体（carrier，返回其 refno 与 world_trans）
/// - 最后对每个负载体独立查询其负几何明细（NegInfo 列表），并在 Rust 侧组装为 ManiGeoTransQuery.neg_ts
///
/// 优点：SQL 更短更直观；局部失败不影响整体；便于调试
pub async fn query_manifold_boolean_operations_simple(
    refno: RefnoEnum,
) -> anyhow::Result<Vec<ManiGeoTransQuery>> {
    use crate::rs_surreal::query_structs::{ManiGeoTransQuery, NegInfo};
    use serde::{Deserialize, Serialize};
    use surrealdb::types::SurrealValue;

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct BaseInfo {
        refno: RefnoEnum,
        sesno: u32,
        noun: String,
        wt: crate::rs_surreal::geometry_query::PlantTransform,
        aabb: crate::types::PlantAabb,
        ts: Vec<TsItem>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct TsItem {
        id: crate::types::RecordId,
        trans: crate::rs_surreal::geometry_query::PlantTransform,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct NegCarrier {
        refno: RefnoEnum,
        wt: crate::rs_surreal::geometry_query::PlantTransform,
    }

    // 1) 基础信息 + 正向几何 ts
    let sql_base = format!(
        r#"
        SELECT
            in AS refno,
            in.sesno AS sesno,
            in.noun AS noun,
            world_trans.d AS wt,
            aabb.d AS aabb,
            (
                SELECT out AS id, trans.d AS trans
                FROM out->geo_relate
                WHERE geo_type IN ["Compound", "Pos"] AND trans.d != NONE
            ) AS ts
        FROM inst_relate:{refno}
        WHERE in.id != NONE AND !bad_bool AND aabb.d != NONE
        "#
    );
    let mut bases: Vec<BaseInfo> = SUL_DB.query_take(&sql_base, 0).await?;
    if bases.is_empty() {
        return Ok(Vec::new());
    }

    // 2) 负载体（Neg 与 Ngmr 两类）
    let pe_key = refno.to_pe_key();
    let sql_carrier_neg = format!(
        r#"
        SELECT in AS refno, world_trans.d AS wt
        FROM array::flatten({pe_key}<-neg_relate.in<-inst_relate)
        WHERE world_trans.d != NONE
        "#
    );
    let mut carriers: Vec<NegCarrier> = SUL_DB.query_take(&sql_carrier_neg, 0).await?;

    let sql_carrier_ngmr = format!(
        r#"
        SELECT in AS refno, world_trans.d AS wt
        FROM array::flatten({pe_key}<-ngmr_relate.in->inst_relate)
        WHERE world_trans.d != NONE
        "#
    );
    carriers.extend(SUL_DB.query_take::<Vec<NegCarrier>>(&sql_carrier_ngmr, 0).await?);

    // 3) 预取本正实体允许的 CataCrossNeg 参照几何（ngmr 列表）
    let allowed_ngmr: Vec<RefnoEnum> = SUL_DB
        .query_take(&format!(r#"SELECT VALUE ngmr FROM {pe_key}<-ngmr_relate"#), 0)
        .await
        .unwrap_or_default();

    // 4) 针对每个负载体查询其负几何明细，组装 neg_ts
    let mut neg_ts: Vec<(RefnoEnum, _, Vec<NegInfo>)> = Vec::with_capacity(carriers.len());
    for c in carriers {
        let sql_neg_geos = if allowed_ngmr.is_empty() {
            format!(
                r#"
                SELECT out AS id, geo_type, para_type ?? "" AS para_type, trans.d AS trans, out.aabb.d AS aabb
                FROM array::flatten({}<-inst_relate).out->geo_relate
                WHERE trans.d != NONE AND geo_type == "Neg"
                "#,
                c.refno.to_pe_key()
            )
        } else {
            let ngmrs = allowed_ngmr
                .iter()
                .map(|x| x.to_pe_key())
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"
                SELECT out AS id, geo_type, para_type ?? "" AS para_type, trans.d AS trans, out.aabb.d AS aabb
                FROM array::flatten({}<-inst_relate).out->geo_relate
                WHERE trans.d != NONE AND (
                    geo_type == "Neg" OR (geo_type == "CataCrossNeg" AND geom_refno IN [{}])
                )
                "#,
                c.refno.to_pe_key(),
                ngmrs
            )
        };
        let infos: Vec<NegInfo> = SUL_DB.query_take(&sql_neg_geos, 0).await.unwrap_or_default();
        neg_ts.push((c.refno, c.wt, infos));
    }

    // 5) 将基础信息扩展为 ManiGeoTransQuery（通常只有一条）
    let mut out: Vec<ManiGeoTransQuery> = Vec::with_capacity(bases.len());
    for b in bases.drain(..) {
        let ts = b.ts.into_iter().map(|t| (t.id, t.trans)).collect();
        out.push(ManiGeoTransQuery {
            refno: b.refno,
            sesno: b.sesno,
            noun: b.noun,
            wt: b.wt,
            aabb: b.aabb,
            ts,
            neg_ts: neg_ts.clone(),
        });
    }

    Ok(out)
}

/// 批量查询正实体到负实体的映射关系
///
/// # Parameters
///
/// * `pos_refnos` - 正实体 refno 列表（为空时查询全库）
///
/// # Returns
///
/// 返回 `HashMap<RefnoEnum, Vec<RefnoEnum>>`，key 是正实体，value 是该正实体对应的所有负实体列表
///
/// # SQL Query
///
/// 查询 neg_relate 和 ngmr_relate 两个表的关系，组装成 pos -> [neg1, neg2, ...] 的映射
pub async fn query_pos_to_negs_mapping(
    pos_refnos: &[RefnoEnum],
) -> anyhow::Result<std::collections::HashMap<RefnoEnum, Vec<RefnoEnum>>> {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosNegRelation {
        pos: RefnoEnum,
        negs: Vec<RefnoEnum>,
    }

    let sql = if pos_refnos.is_empty() {
        // 全库扫描：从 pe 表查询所有存在负实体关系的正实体
        r#"
            SELECT
                id as pos,
                array::distinct(array::union(<-neg_relate.in, <-ngmr_relate.in)) as negs
            FROM pe
            WHERE array::len(array::union(<-neg_relate.in, <-ngmr_relate.in)) > 0
        "#.to_string()
    } else {
        // 指定范围查询
        let refno_keys = pos_refnos
            .iter()
            .map(|r| r.to_pe_key())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"
            SELECT
                id as pos,
                array::distinct(array::union(<-neg_relate.in, <-ngmr_relate.in)) as negs
            FROM [{refno_keys}]
            WHERE array::len(array::union(<-neg_relate.in, <-ngmr_relate.in)) > 0
            "#
        )
    };

    let relations: Vec<PosNegRelation> = SUL_DB.query_take(&sql, 0).await?;

    let mut mapping = HashMap::with_capacity(relations.len());
    for rel in relations {
        mapping.insert(rel.pos, rel.negs);
    }

    Ok(mapping)
}

/// 批量查询布尔运算数据（基于 pos->negs 映射）
///
/// # Parameters
///
/// * `pos_refnos` - 正实体 refno 列表（为空时查询全库）
///
/// # Returns
///
/// 返回 `Vec<ManiGeoTransQuery>` 列表
///
/// # 实现策略
///
/// 1. 先调用 `query_pos_to_negs_mapping` 获取 pos->negs 映射
/// 2. 对每个正实体并发查询其完整的布尔运算数据
/// 3. 使用 `query_manifold_boolean_operations_simple` 避免复杂嵌套查询
pub async fn query_manifold_boolean_operations_batch(
    pos_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<ManiGeoTransQuery>> {
    // Step 1: 获取所有需要布尔运算的正实体
    let targets = if pos_refnos.is_empty() {
        query_manifold_boolean_targets_in(&[]).await?
    } else {
        query_manifold_boolean_targets_in(pos_refnos).await?
    };

    if targets.is_empty() {
        return Ok(Vec::new());
    }

    // Step 2: 并发查询每个正实体的布尔运算数据
    let mut handles = Vec::with_capacity(targets.len());

    for target in targets {
        let handle = tokio::spawn(async move {
            query_manifold_boolean_operations_simple(target).await
        });
        handles.push(handle);
    }

    // Step 3: 收集结果
    let mut all_results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(mut results)) => {
                all_results.extend(results.drain(..));
            }
            Ok(Err(e)) => {
                eprintln!("查询布尔运算数据失败: {:?}", e);
            }
            Err(e) => {
                eprintln!("任务执行失败: {:?}", e);
            }
        }
    }

    Ok(all_results)
}

