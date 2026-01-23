use crate::parsed_data::{CateAxisParam, TubiInfoData};
use crate::pdms_types::PdmsGenericType;
use crate::rs_surreal::geometry_query::PlantTransform;
use crate::types::PlantAabb;
use crate::{RefU64, RefnoEnum, SUL_DB, SurrealQueryExt};
use bevy_transform::components::Transform;
use dashmap::DashMap;
use glam::Vec3;
use itertools::Itertools;
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TubiInstQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    #[serde(default)]
    pub world_aabb: Option<PlantAabb>,
    pub world_trans: Transform,
    pub geo_hash: String,
}

///查询离开点和到达点, local 为true时，表示使用局部坐标
pub async fn query_arrive_leave_points(
    refnos: impl IntoIterator<Item = &RefU64>,
    local: bool,
) -> anyhow::Result<DashMap<RefU64, [CateAxisParam; 2]>> {
    let pes = crate::join_pe_keys(refnos.into_iter());
    if pes.is_empty() {
        return Ok(DashMap::new());
    }
    let sql = format!(
        r#"
             select value [
                    in,
                    world_trans.d,
                    (select * from out.ptset where number=$parent.in.refno.ARRI)[0],
                    (select * from out.ptset where number=$parent.in.refno.LEAV)[0]
                ]
              from array::flatten([{}][? owner.noun in ['BRAN', 'HANG']]->inst_relate) where world_trans.d!=none
             "#,
        pes
    );

    println!("query_arrive_leave_points sql: {}", sql);

    let rows: Vec<(
        RefU64,
        PlantTransform,
        Option<CateAxisParam>,
        Option<CateAxisParam>,
    )> = SUL_DB.query_take(&sql, 0).await?;
    let mut map = DashMap::new();
    for (refno, trans, arri, leav) in rows {
        if arri.is_none() || leav.is_none() {
            continue;
        }
        let mut pts = [arri.unwrap(), leav.unwrap()];
        if !local {
            let t: Transform = *trans;
            pts[0].transform(&t);
            pts[1].transform(&t);
        }
        map.insert(refno, pts);
    }
    Ok(map)
}

pub async fn query_arrive_leave_points_of_component(
    refnos: impl IntoIterator<Item = &RefU64>,
) -> anyhow::Result<DashMap<RefnoEnum, [CateAxisParam; 2]>> {
    let pes = crate::join_pe_keys(refnos.into_iter());
    if pes.is_empty() {
        return Ok(DashMap::new());
    }
    let sql = format!(
        r#"
             select value [in,
                (select * from out.ptset where number=$parent.in.refno.ARRI)[0],
                (select * from out.ptset where number=$parent.in.refno.LEAV)[0]
            ] from array::flatten([{}][? owner.noun in ['BRAN', 'HANG']]->inst_relate)
             "#,
        pes
    );

    // println!("query_arrive_leave_points_of_component sql: {}", sql);

    let rows: Vec<(RefnoEnum, Option<CateAxisParam>, Option<CateAxisParam>)> =
        SUL_DB.query_take(&sql, 0).await?;
    let mut map = DashMap::new();
    for (refno, arri, leav) in rows {
        if arri.is_none() || leav.is_none() {
            continue;
        }
        let mut pts = [arri.unwrap(), leav.unwrap()];
        map.insert(refno, pts);
    }
    Ok(map)
}

pub async fn query_arrive_leave_points_of_branch(
    branch_refno: RefnoEnum,
) -> anyhow::Result<DashMap<RefnoEnum, [CateAxisParam; 2]>> {
    // 同时获取 ptset 和 world_trans，将局部坐标转换为世界坐标
    let sql = format!(
        r#"
             select value [id,
                (->inst_relate.world_trans.d ?? ->tubi_relate.world_trans.d)[0],
                (select * from (->inst_relate.out.ptset)[0] where number=$parent.refno.ARRI)[0],
                (select * from (->inst_relate.out.ptset)[0] where number=$parent.refno.LEAV)[0]
            ] from {}.children
             "#,
        branch_refno.to_pe_key()
    );

    // println!("query_arrive_leave_points_of_branch sql: {}", sql);

    let rows: Vec<(
        RefnoEnum,
        Option<PlantTransform>,
        Option<CateAxisParam>,
        Option<CateAxisParam>,
    )> = SUL_DB.query_take(&sql, 0).await?;
    let mut map = DashMap::new();
    for (refno, world_trans, arri, leav) in rows {
        if arri.is_none() || leav.is_none() {
            continue;
        }
        let mut pts = [arri.unwrap(), leav.unwrap()];
        // 应用 world_trans 转换为世界坐标
        if let Some(trans) = world_trans {
            let t: Transform = *trans;
            pts[0].transform(&t);
            pts[1].transform(&t);
        }
        map.insert(refno, pts);
    }
    Ok(map)
}

// ============================================================================
// tubi_info 查询函数
// ============================================================================

/// 查询 BRAN/HANG 下所有子元件的 tubi_info 和 world_transform
///
/// 返回: Vec<(元件 refno, tubi_info_id, world_transform)>
#[derive(Serialize, Deserialize, Debug, Clone, SurrealValue)]
pub struct BranchChildTubiQuery {
    pub refno: RefnoEnum,
    pub tubi_info_id: Option<String>,
    pub world_trans: Option<PlantTransform>,
}

/// 查询 branch 下所有子元件的 tubi_info 关联信息
pub async fn query_branch_children_tubi_info(
    branch_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<BranchChildTubiQuery>> {
    if branch_refnos.is_empty() {
        return Ok(vec![]);
    }

    let branch_keys = branch_refnos.iter().map(|r| r.to_pe_key()).join(",");

    let sql = format!(
        r#"
        SELECT 
            id as refno,
            (->inst_relate->inst_info.tubi_info)[0] as tubi_info_id,
            (->inst_relate.world_trans)[0] as world_trans
        FROM array::flatten([{}].children)
        WHERE (->inst_relate->inst_info.tubi_info)[0] != NONE
        "#,
        branch_keys
    );

    let rows: Vec<BranchChildTubiQuery> = SUL_DB.query_take(&sql, 0).await.unwrap_or_default();
    Ok(rows)
}

/// 批量查询 tubi_info 记录
pub async fn query_tubi_info_by_ids(
    ids: &[String],
) -> anyhow::Result<HashMap<String, TubiInfoData>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    const BATCH_SIZE: usize = 500;
    let mut result = HashMap::new();

    for chunk in ids.chunks(BATCH_SIZE) {
        let id_list: String = chunk
            .iter()
            .map(|id| format!("tubi_info:⟨{}⟩", id))
            .join(",");

        let sql = format!("SELECT * FROM tubi_info WHERE id IN [{}];", id_list);

        let rows: Vec<TubiInfoData> = SUL_DB.query_take(&sql, 0).await.unwrap_or_default();
        for row in rows {
            result.insert(row.id.clone(), row);
        }
    }

    Ok(result)
}

/// 查询 branch 下元件的 arrive/leave 点（基于 tubi_info 表）
///
/// 返回: DashMap<元件 refno, [arrive CateAxisParam, leave CateAxisParam]>
/// 注意：返回的是 world 坐标（已应用 world_transform）
pub async fn query_arrive_leave_from_tubi_info(
    branch_refnos: &[RefnoEnum],
) -> anyhow::Result<DashMap<RefnoEnum, [CateAxisParam; 2]>> {
    // 1. 查询子元件的 tubi_info_id 和 world_trans
    let children = query_branch_children_tubi_info(branch_refnos).await?;

    if children.is_empty() {
        return Ok(DashMap::new());
    }

    // 2. 收集所有 tubi_info_id
    let tubi_info_ids: Vec<String> = children
        .iter()
        .filter_map(|c| c.tubi_info_id.clone())
        .collect();

    // 3. 批量查询 tubi_info
    let tubi_info_map = query_tubi_info_by_ids(&tubi_info_ids).await?;

    // 4. 组装结果（应用 world_transform）
    let result = DashMap::new();
    for child in children {
        let Some(tubi_id) = child.tubi_info_id else {
            continue;
        };
        let Some(tubi_info) = tubi_info_map.get(&tubi_id) else {
            continue;
        };

        // 转换为 CateAxisParam
        let mut arrive = tubi_info.arrive.to_axis_param(child.refno);
        let mut leave = tubi_info.leave.to_axis_param(child.refno);

        // 应用 world_transform
        if let Some(trans) = child.world_trans {
            let t: Transform = *trans;
            arrive.transform(&t);
            leave.transform(&t);
        }

        result.insert(child.refno, [arrive, leave]);
    }

    Ok(result)
}
