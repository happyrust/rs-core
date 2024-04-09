use bevy_transform::components::Transform;
use dashmap::DashMap;
use glam::Vec3;
use serde_derive::{Deserialize, Serialize};
use crate::{RefU64, SUL_DB};
use serde_with::serde_as;
use parry3d::bounding_volume::Aabb;
use crate::basic::aabb::ParryAabb;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::PdmsGenericType;

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
pub async fn query_arrive_leave_points(refnos: impl IntoIterator<Item = &RefU64>,
                                       local: bool) -> anyhow::Result<DashMap<RefU64, [CateAxisParam; 2]>> {
    let pes: String = refnos.into_iter().map(|x| x.to_pe_key()).collect::<Vec<_>>().join(",");
    let mut response = SUL_DB
        .query(format!(r#"
             select value [
                    in,
                    world_trans.d,
                    (select * from object::values(out.ptset) where number=$parent.in.refno.ARRI)[0],
                    (select * from object::values(out.ptset) where number=$parent.in.refno.LEAV)[0]
                ]
              from array::flatten([{}][? owner.noun in ['BRAN', 'HANG']]->inst_relate) where world_trans.d!=none
             "#, pes))
        .await?;

    let result: Vec<(RefU64, Transform, Option<CateAxisParam>, Option<CateAxisParam>)> = response.take(0)?;
    // dbg!(&r);
    let mut map = DashMap::new();
    for (refno, trans, a_pt, l_pt) in result {
        if a_pt.is_none() || l_pt.is_none() {
            continue;
        }
        let mut pts = [a_pt.unwrap(), l_pt.unwrap()];
        if !local {
            pts[0].transform(&trans);
            pts[1].transform(&trans);
        }
        map.insert(refno, pts);
    }
    Ok(map)
}