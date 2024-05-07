use crate::basic::aabb::ParryAabb;
use crate::pdms_types::PdmsGenericType;
use crate::{get_inst_relate_keys, RefU64, SUL_DB};
use bevy_transform::components::Transform;
use glam::Vec3;
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TubiInstQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    pub generic: Option<String>,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub geo_hash: String,
}

pub async fn query_tubi_insts_by_brans(
    bran_refnos: &[RefU64],
) -> anyhow::Result<Vec<TubiInstQuery>> {
    let pes: String = bran_refnos
        .iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        r#"
             select in.id as refno, (in->pe_owner->pe.noun)[0] as generic, aabb.d as world_aabb, world_trans.d as world_trans, meta::id(out) as geo_hash
                from  array::flatten([{}]->tubi_relate) where leave.id != none and aabb.d != none
             "#,
        pes
    );
    // println!("Query tubi insts: {}", &sql);
    let mut response = SUL_DB.query(&sql).await?;

    let r = response.take::<Vec<TubiInstQuery>>(0)?;
    Ok(r)
}

pub async fn query_tubi_insts_by_flow(refnos: &[RefU64]) -> anyhow::Result<Vec<TubiInstQuery>> {
    let pes: String = refnos
        .iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let mut response = SUL_DB
        .query(format!(r#"
        array::group(array::complement(select value
        (select in.id as refno, (in->pe_owner->pe.noun)[0] as generic, aabb.d as world_aabb, world_trans.d as world_trans, meta::id(out) as geo_hash
            from tubi_relate where leave=$parent.id or arrive=$parent.id)
                from [{}] where owner.noun in ['BRAN', 'HANG'] and aabb.d!=none, [none]))
             "#, pes))
        .await?;

    let r = response.take::<Vec<TubiInstQuery>>(0)?;
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
    pub owner: RefU64,
    pub insts: Vec<ModelHashInst>,
    pub generic: PdmsGenericType,
    pub world_trans: Transform,
    pub world_aabb: ParryAabb,
    pub ptset: Vec<Vec3>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeomInstQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    pub owner: RefU64,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub insts: Vec<ModelHashInst>,
    pub generic: String,
    pub pts: Option<Vec<Vec3>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeomPtsQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    pub world_trans: Transform,
    pub world_aabb: Aabb,
    pub pts_group: Vec<(Transform, Option<Vec<Vec3>>)>,
}

pub async fn query_insts(
    refnos: impl IntoIterator<Item = &RefU64>,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();
    let inst_keys = get_inst_relate_keys(&refnos);

    let sql = format!(r#"
                    select in.id as refno, in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
            if neg_refnos != none && $parent.booled {{ [{{ "geo_hash": meta::id(in.id) }}] }} else {{ (select trans.d as transform, meta::id(out) as geo_hash from out->geo_relate where trans.d != none and geo_type='Pos')  }} as insts
            from {inst_keys} where aabb.d != none
            "#);
    // println!("Query insts: {}", &sql);
    let mut response = SUL_DB
        .query(sql)
        .await?;
    let mut geom_insts: Vec<GeomInstQuery> = response.take(0).unwrap();

    Ok(geom_insts)
}
