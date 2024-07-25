use crate::consts::MAX_INSERT_LENGTH;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode};
use crate::pe::SPdmsElement;
use crate::table::ToTable;
use crate::tool::db_tool::db1_dehash;
use crate::tool::math_tool::*;
use crate::{graph::QUERY_DEEP_CHILDREN_REFNOS, types::*};
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use cached::Cached;
use dashmap::DashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::{BTreeMap, HashMap};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;


pub async fn query_db_max_version(db_num: u32) -> anyhow::Result<u32> {
    let mut response = SUL_DB
        .query(format!(
            r#"object::values((select math::max(sesno) from pe where dbnum={db_num} group all)[0])[0];"#,
        ))
        .await?;
    let max_version: Option<u32> = response.take(0)?;
    Ok(max_version.unwrap_or_default())
}