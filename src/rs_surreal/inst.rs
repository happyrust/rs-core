use bevy_transform::components::Transform;
use serde_derive::{Deserialize, Serialize};
use crate::{RefU64, SUL_DB};
use serde_with::serde_as;
use parry3d::bounding_volume::Aabb;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TubiInstQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub geo_hash: String,
}

pub async fn query_tubi_insts(bran_refnos: &[RefU64]) -> anyhow::Result<Vec<TubiInstQuery>> {
    let pes: String = bran_refnos.iter().map(|x| x.to_pe_key()).collect::<Vec<_>>().join(",");
    let mut response = SUL_DB
        .query(format!(r#"
             select in.id as refno, aabb.d as world_aabb, world_trans.d as world_trans, meta::id(out) as geo_hash
                from  array::flatten([{}]->tubi_relate) where leave.id != none
             "#, pes))
        .await?;

    let r = response.take::<Vec<TubiInstQuery>>(0)?;
    Ok(r)
}

