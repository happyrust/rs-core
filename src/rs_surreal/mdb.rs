use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use indexmap::IndexMap;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::E;
use std::sync::Mutex;

#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Hash, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum DBType {
    DESI = 1,
    CATA = 2,
    PROP = 3,
    ISOD = 4,
    PADD = 5,
    DICT = 6,
    ENGI = 7,
    SCHE = 14,
    UNSET,
}

/// Executes a query on the SUL_DB database to retrieve information from MDB and DB tables.
///
/// # Arguments
///
/// * `mdb` - The name of the MDB to query.
/// * `db_type` - The type of DB to filter by.
///
/// # Returns
///
/// The response containing the refno, noun, name, owner, and children_count fields from the query.
#[cached(result = true)]
pub async fn get_mdb_world_site_ele_nodes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<EleTreeNode>> {
    let db_type: u8 = module.into();
    let mut response = SUL_DB
        .query(" \
        let $dbnos = array::intersect((select value CURD.refno.DBNO from only MDB where NAME=$mdb limit 1), select value DBNO from DB where STYP=$db_type); \
        let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o)); \
        let $b = array::flatten(select value (select value in.* from (select * from <-pe_owner order by order_num) where in.id!=none) from $a ); \
        select refno, noun, name, owner, array::len(select value in from <-pe_owner) as children_count from $b
        ")
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    // dbg!(&response);
    let nodes: Vec<EleTreeNode> = response.take(3)?;
    dbg!(nodes.len());
    //检查名称，如果没有给名字的，需要给上默认值, todo 后续如果是删除了又增加，名称后面的数字可能会继续增加
    Ok(nodes)
}

// //let $dbnos = select value CURD.refno.DBNO from only MDB where NAME="/ALL" limit 1
//通过surql查询pe数据
#[cached(result = true)]
pub async fn get_mdb_world_site_pes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<SPdmsElement>> {
    let db_type: u8 = module.into();
    // let sql = format!("let $dbnos = array::intersect((select value CURD.refno.DBNO from only MDB where NAME={} limit 1), select value DBNO from DB where STYP={}); \
    //     let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o)); \
    //     array::flatten(select value (select value in.* from (select * from <-pe_owner order by order_num) where in.id!=none) from $a )",mdb,db_type);
    let mut response = SUL_DB
        .query(" \
        let $dbnos = array::intersect((select value CURD.refno.DBNO from only MDB where NAME=$mdb limit 1), select value DBNO from DB where STYP=$db_type); \
        let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o)); \
        array::flatten(select value (select value in.* from (select * from <-pe_owner order by order_num) where in.id!=none) from $a )")
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    // let mut response = SUL_DB
    //     .query(sql)
    //     .await?;
    let pe: Vec<SPdmsElement> = response.take(2)?;
    Ok(pe)
}

/// Represents the response obtained from the database query.
#[cached(result = true)]
pub async fn get_world(mdb: String) -> anyhow::Result<Option<SPdmsElement>> {
    let mut response = SUL_DB
        .query(
            " \
            let $f = (select value CURD.refno.DBNO from only MDB where NAME=$mdb limit 1)[0]; \
            (select value REFNO.* from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        )
        .bind(("mdb", mdb))
        .await
        .unwrap();
    let pe: Option<SPdmsElement> = response.take(1)?;
    Ok(pe)
}

/// Represents the response obtained from the database query.
#[cached(result = true)]
pub async fn get_world_refno(mdb: String) -> anyhow::Result<RefU64> {
    let mdb_name = if mdb.starts_with('/') {
        mdb.clone()
    } else {
        format!("/{}", mdb)
    };
    let mut response = SUL_DB
        .query(
            " \
            let $f = (select value CURD.refno.DBNO from only MDB where NAME=$mdb limit 1)[0]; \
            (select value REFNO from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        )
        .bind(("mdb", mdb_name))
        .await
        .unwrap();
    let id: Option<RefU64> = response.take(1)?;
    Ok(id.unwrap_or_default())
}
