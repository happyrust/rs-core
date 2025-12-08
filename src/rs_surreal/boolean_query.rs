use surrealdb::types::{self as surrealdb_types, SurrealValue};

use crate::rs_surreal::query_structs::{CataNegGroup, GmGeoData};
use crate::types::RefnoEnum;
use crate::{get_inst_relate_keys, SurrealQueryExt, SUL_DB};

/// Boolean operation query functions
///
/// This module provides query functions for boolean operations on geometry,
/// including catalog negative boolean groups and negative entity queries.

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
/// - Optionally filters by `bool_status != 'Success'` if not replacing
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
        // 仅处理尚未成功布尔运算的实体
        sql.push_str("and (bool_status != 'Success' or bool_status = none)");
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

/// 查询正实体关联的所有负实体 refnos
///
/// 使用 SurrealDB 函数 `fn::query_negative_entities` 简化查询
///
/// # Parameters
///
/// * `refno` - 正实体 refno
///
/// # Returns
///
/// 返回关联的负实体 refnos 列表
pub async fn query_negative_entities(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!(
        r#"RETURN fn::query_negative_entities({})"#,
        refno.to_pe_key()
    );
    SUL_DB.query_take(&sql, 0).await
}

/// 批量查询正实体关联的负实体
///
/// # Parameters
///
/// * `refnos` - 正实体 refno 列表
///
/// # Returns
///
/// 返回 `HashMap<RefnoEnum, Vec<RefnoEnum>>`，key 是正实体，value 是该正实体对应的所有负实体列表
pub async fn query_negative_entities_batch(
    refnos: &[RefnoEnum],
) -> anyhow::Result<std::collections::HashMap<RefnoEnum, Vec<RefnoEnum>>> {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosNegRelation {
        pos: RefnoEnum,
        negs: Vec<RefnoEnum>,
    }

    if refnos.is_empty() {
        return Ok(HashMap::new());
    }

    let refno_keys = refnos
        .iter()
        .map(|r| r.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        r#"
        SELECT id AS pos, fn::query_negative_entities(id) AS negs
        FROM [{refno_keys}]
        WHERE array::len(fn::query_negative_entities(id)) > 0
        "#
    );

    let relations: Vec<PosNegRelation> = SUL_DB.query_take(&sql, 0).await?;

    let mut mapping = HashMap::with_capacity(relations.len());
    for rel in relations {
        mapping.insert(rel.pos, rel.negs);
    }

    Ok(mapping)
}

/// 查询需要布尔运算的正实体（存在负实体关联）
///
/// # Parameters
///
/// * `refnos` - 正实体 refno 列表（为空时全库扫描）
///
/// # Returns
///
/// 返回存在负实体关联的正实体 id 列表
pub async fn query_boolean_targets(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        let sql = r#"
            SELECT VALUE id FROM pe
            WHERE array::len(fn::query_negative_entities(id)) > 0
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
        WHERE array::len(fn::query_negative_entities(id)) > 0
        "#
    );

    SUL_DB.query_take(&sql, 0).await
}

