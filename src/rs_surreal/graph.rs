use crate::noun_graph::*;
use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use surrealdb::method::Stats;

#[inline]
pub async fn query_filter_all_bran_hangs(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    query_filter_deep_children(refno, &["BRAN", "HANG"]).await
}

/// Represents the SQL query used to retrieve values from a database.
/// The query is constructed dynamically based on the provided parameters.
/// It selects the `refno` values from a flattened array of objects,
/// where the `noun` values match the specified list of nouns.
pub async fn query_filter_deep_children(
    refno: RefU64,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefU64>> {
    let end_noun = super::get_type_name(refno).await?;
    dbg!(&end_noun);
    if let Some(relate_sql) = gen_noun_incoming_relate_sql(&end_noun, nouns) {
        // dbg!(&relate_sql);
        let nouns_str = nouns
            .iter()
            .map(|&s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only pe:{refno})) where noun in [{nouns_str}]",
        );
        // dbg!(&sql);
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            let execution_time = stats.execution_time;
            dbg!(&execution_time);
            // let s: Vec<RefU64> = result?;
            return Ok(result);
        }
    }
    Ok(vec![])
}

pub async fn query_multi_filter_deep_children(
    refnos: &[RefU64],
    nouns: &[&str],
) -> anyhow::Result<HashSet<RefU64>> {
    let mut result = HashSet::new();
    for &refno in refnos {
        let mut children = query_filter_deep_children(refno, nouns).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

//TODO: 使用统一的方法调用，查询path的filter
pub async fn query_filter_ancestors(
    refno: RefU64,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefU64>> {
    let start_noun = super::get_type_name(refno).await?;
    dbg!(&start_noun);
    if let Some(relate_sql) = gen_noun_outcoming_relate_sql(&start_noun, nouns) {
        // dbg!(&relate_sql);
        let nouns_str = nouns
            .iter()
            .map(|&s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only pe:{refno})) where noun in [{nouns_str}]",
        );
        // dbg!(&sql);
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            let execution_time = stats.execution_time;
            dbg!(&execution_time);
            // let s: Vec<RefU64> = result?;
            return Ok(result);
        }
    }
    Ok(vec![])
}