use crate::basic::aabb::ParryAabb;
use crate::pdms_types::PdmsGenericType;
use crate::{get_inst_relate_keys, RefU64, RefnoEnum, SUL_DB};
use bevy_transform::components::Transform;
use chrono::{DateTime, Local, NaiveDateTime};
use glam::{DVec3, Vec3};
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
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
    pub date: surrealdb::sql::Datetime,
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

    let r = response.take::<Vec<TubiInstQuery>>(0)?;
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
                from [{}] where owner.noun in ['BRAN', 'HANG'], [none]))
             "#,
        pes
    );
    // println!("Sql query_tubi_insts_by_flow: {}", &sql);
    let mut response = SUL_DB.query(sql).await?;

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
    pub owner: RefnoEnum,
    pub old_refno: Option<RefnoEnum>,
    pub insts: Vec<ModelHashInst>,
    pub generic: PdmsGenericType,
    pub world_trans: Transform,
    pub world_aabb: ParryAabb,
    pub ptset: Vec<Vec3>,
    pub is_bran_tubi: bool,
    pub date: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeomInstQuery {
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    pub old_refno: Option<RefnoEnum>,
    pub owner: RefnoEnum,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub insts: Vec<ModelHashInst>,
    pub generic: String,
    pub pts: Option<Vec<Vec3>>,
    pub date: surrealdb::sql::Datetime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeomPtsQuery {
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    pub world_trans: Transform,
    pub world_aabb: Aabb,
    pub pts_group: Vec<(Transform, Option<Vec<DVec3>>)>,
}


/// 根据最新refno查询最新insts
pub async fn query_insts(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();

    //需要区分历史模型和当前最新模型

    let inst_keys = get_inst_relate_keys(&refnos);

    let sql = format!(
        r#"
            select
                in.id as refno,
                in.old_pe as old_refno,
                in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
                if booled_id != none {{ [{{ "geo_hash": booled_id }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none && geo_type='Pos')  }} as insts,
                fn::ses_date(in.id) as date
            from {inst_keys} where aabb.d != none
        "#
    );
    // println!("Query insts sql: {}", &sql);
    let mut response = SUL_DB.query(sql).await?;
    let mut geom_insts: Vec<GeomInstQuery> = response.take(0)?;
    // dbg!(&geom_insts);

    Ok(geom_insts)
}

// 根据历史refno查询历史insts
// pub async fn query_history_insts(
//     refnos: impl IntoIterator<Item = &RefnoEnum>,
// ) -> anyhow::Result<Vec<GeomInstQuery>> {
//     let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();

//     //需要区分历史模型和当前最新模型

//     let inst_keys = get_inst_relate_keys(&refnos);

//     let sql = format!(
//         r#"
//             select
//                 in.id as refno,
//                 in.old_pe as old_refno,
//                 in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
//                 if booled_id != none {{ [{{ "geo_hash": booled_id }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && out.meshed && trans.d != none && geo_type='Pos')  }} as insts,
//                 fn::ses_date(in.id) as date
//             from {inst_keys} where aabb.d != none
//         "#
//     );
//     // println!("Query insts sql: {}", &sql);
//     let mut response = SUL_DB.query(sql).await?;
//     let mut geom_insts: Vec<GeomInstQuery> = response.take(0)?;
//     // dbg!(&geom_insts);

//     Ok(geom_insts)
// }

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
