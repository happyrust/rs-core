use crate::noun_graph::*;
use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{NamedAttrMap, RefU64, rs_surreal};
use crate::{SUL_DB, SurlValue};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use surrealdb::method::Stats;
use cached::proc_macro::cached;


#[inline]
#[cached(result = true)]
pub async fn query_filter_all_bran_hangs(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    query_filter_deep_children(refno, vec!["BRAN".into(), "HANG".into()]).await
}

/// Represents the SQL query used to retrieve values from a database.
/// The query is constructed dynamically based on the provided parameters.
/// It selects the `refno` values from a flattened array of objects,
/// where the `noun` values match the specified list of nouns.
#[cached(result = true)]
pub async fn query_filter_deep_children(
    refno: RefU64,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<RefU64>> {
    let end_noun = super::get_type_name(refno).await?;
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let nouns_slice = nouns.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(relate_sql) = gen_noun_incoming_relate_sql(&end_noun, &nouns_slice) {
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only pe:{refno})) where noun in [{nouns_str}]",
        );
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            return Ok(result);
        }
    }
    Ok(vec![])
}

#[cached(result = true)]
pub async fn query_deep_children_skip_exist_inst(
    refno: RefU64,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<RefU64>> {
    let end_noun = super::get_type_name(refno).await?;
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let nouns_slice = nouns.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(relate_sql) = gen_noun_incoming_relate_sql(&end_noun, &nouns_slice) {
        let sql = format!(
            r#"select value refno from array::flatten(object::values(select {relate_sql} from only pe:{refno}))
             where array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0 and noun in [{nouns_str}]"#,
        );
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            return Ok(result);
        }
    }
    Ok(vec![])
}

#[cached(result = true)]
pub async fn query_multi_filter_deep_children(
    refnos: Vec<RefU64>,
    nouns: Vec<String>,
) -> anyhow::Result<HashSet<RefU64>> {
    let mut result = HashSet::new();
    for refno in refnos {
        let mut children = query_filter_deep_children(refno, nouns.clone()).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

#[cached(result = true)]
pub async fn query_multi_deep_children_skip_exist_inst(
    refnos: Vec<RefU64>,
    nouns: Vec<String>,
) -> anyhow::Result<HashSet<RefU64>> {
    let mut result = HashSet::new();
    for refno in refnos {
        let mut children = query_deep_children_skip_exist_inst(refno, nouns.clone()).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

#[cached(result = true)]
pub async fn query_filter_ancestors(
    refno: RefU64,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<RefU64>> {
    let start_noun = super::get_type_name(refno).await?;
    // dbg!(&start_noun);
    let nouns_str = nouns
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(",");
    let nouns_slice = nouns.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(relate_sql) = gen_noun_outcoming_relate_sql(&start_noun,  &nouns_slice) {
        // dbg!(&relate_sql);
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only pe:{refno})) where noun in [{nouns_str}]",
        );
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            let execution_time = stats.execution_time;
            return Ok(result);
        }
    }
    Ok(vec![])
}