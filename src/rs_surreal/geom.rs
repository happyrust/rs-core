use std::collections::{BTreeMap, HashMap};
use crate::pdms_types::{EleTreeNode, VISBILE_GEO_NOUNS};
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use cached::proc_macro::cached;
use std::sync::Mutex;



///通过surql查询pe数据
#[cached(result = true)]
pub async fn query_deep_inst_info_refnos(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let types = super::get_self_and_owner_type_name(refno).await?;
    dbg!(&types);
    if types[1] == "BRAN" || types[1] == "HANG" {
        return Ok(vec![refno]);
    }
    if types[0] == "BRAN" || types[0] == "HANG" {
        let children_refnos = super::get_children_refnos(refno).await?;
        return Ok(children_refnos);
    }
    let branch_refnos = super::query_refno_deep_children(
        refno,
        &["BRAN", "HANG"],
    ).await?;

    dbg!(&branch_refnos);

    let mut target_refnos = super::query_multi_children_refnos(&branch_refnos).await?;

    let visible_refnos = super::query_refno_deep_children(
        refno,
        &VISBILE_GEO_NOUNS,
    ).await?;
    
    target_refnos.extend(visible_refnos);
    Ok(target_refnos)
}
