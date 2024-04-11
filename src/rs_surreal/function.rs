use cached::proc_macro::cached;
use crate::{NamedAttrMap, RefU64, SUL_DB, SurlValue};

pub async fn define_common_functions() -> anyhow::Result<()> {
    SUL_DB
        .query(include_str!("schemas/functions/common.surql"))
        .await?;
    Ok(())
}