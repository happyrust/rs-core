use crate::basic::aabb::ParryAabb;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::PdmsGenericType;
use crate::rs_surreal::geometry_query::PlantTransform;
use crate::{RefU64, RefnoEnum, SUL_DB, SurrealQueryExt};
use bevy_transform::components::Transform;
use dashmap::DashMap;
use glam::Vec3;
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use surrealdb::types::SurrealValue;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TubiInstQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    pub world_aabb: Aabb,
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
             select value [id,
                (select * from type::record("inst_info", cata_hash).ptset where number=$parent.refno.ARRI)[0],
                (select * from type::record("inst_info", cata_hash).ptset where number=$parent.refno.LEAV)[0]
            ] from array::flatten([{}][? owner.noun in ['BRAN', 'HANG']])
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
    let sql = format!(
        r#"
             select value [id,
                (select * from type::record("inst_info", cata_hash).ptset where number=$parent.refno.ARRI)[0],
                (select * from type::record("inst_info", cata_hash).ptset where number=$parent.refno.LEAV)[0]
            ] from {}.children
             "#,
        branch_refno.to_pe_key()
    );

    // println!("query_arrive_leave_points_of_branch sql: {}", sql);

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
