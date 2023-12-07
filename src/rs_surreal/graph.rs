use crate::noun_graph::{gen_noun_outcoming_relate_path, NOUN_GRAPH};
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
pub async fn query_all_bran_hangs(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    query_refno_deep_children(refno, &["BRAN", "HANG"]).await
}

/// 查询节点下所有的bran/hanger 节点
pub async fn query_refno_deep_children(
    refno: RefU64,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefU64>> {
    let end_noun = super::get_type_name(refno).await?;
    if let Some(relate_sql) = gen_noun_outcoming_relate_path(&end_noun, nouns) {
        // dbg!(&relate_sql);
        let nouns_str = nouns
            .iter()
            .map(|&s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only pe:{refno})) where noun in [{nouns_str}]",
        );
        dbg!(&sql);
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, result)) = response.take(0) {
            let execution_time = stats.execution_time;
            dbg!(&execution_time);
            let s: Vec<RefU64> = result?;
            return Ok(s);
        }
    }
    Ok(vec![])
}


pub async fn query_refnos_deep_children(
    refnos: &[RefU64],
    nouns: &[&str],
) -> anyhow::Result<HashSet<RefU64>> {
    let mut result = HashSet::new();
    for &refno in refnos {
        let mut children = query_refno_deep_children(refno, nouns).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}