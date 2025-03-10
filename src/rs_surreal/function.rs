use crate::{NamedAttrMap, RefU64, SurlValue, SUL_DB};
use cached::proc_macro::cached;

pub async fn define_common_functions() -> anyhow::Result<()> {
    SUL_DB
        .query(include_str!("schemas/functions/common.surql"))
        .await?;
    SUL_DB
        .query(include_str!("material_list/common.surql"))
        .await?;
    #[cfg(not(feature = "hh"))]
    SUL_DB
        .query(include_str!("schemas/fn_query_room_code.surql"))
        .await?;
    #[cfg(feature = "hh")]
    SUL_DB.query(include_str!("schemas/fn_query_room_code_hh.surql")).await?;
    SUL_DB
        .query(include_str!("schemas/get_room_nodes.surql"))
        .await?;
    SUL_DB
        .query(include_str!("schemas/status/init_status.surql"))
        .await?;
    Ok(())
}
