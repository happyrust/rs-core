use crate::{NamedAttrMap, RefU64, SurlValue, SUL_DB};
use cached::proc_macro::cached;

pub async fn define_common_functions() -> anyhow::Result<()> {
    SUL_DB
        .query(include_str!("schemas/functions/common.surql"))
        .await?;
    SUL_DB
        .query(include_str!("material_list/common.surql"))
        .await?;
    SUL_DB
        .query(include_str!("material_list/gy/gy_common.surql"))
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

/// 定义数据库编号事件
/// 
/// 当创建新的 pe 记录时,会触发此事件来更新 dbnum_info_table 表中的信息
/// 
/// # 错误
/// 
/// 如果数据库操作失败,将返回错误
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    SUL_DB
        .query(r#"
        DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
            -- 获取当前记录的 dbnum
            LET $dbnum = $value.dbnum;
            LET $id = record::id($value.id);
            let $ref_0 = array::at($id, 0);
            let $ref_1 = array::at($id, 1);
            let $is_delete = $value.deleted and $event = "UPDATE";
            let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };
            -- 根据事件类型处理  type::thing("dbnum_info_table", $ref_0)
            IF $event = "CREATE"   {
                UPSERT type::thing('dbnum_info_table', $ref_0) SET
                    dbnum = $dbnum,
                    count = count?:0 + 1,
                    sesno = $max_sesno,
                    max_ref1 = $ref_1;
            } ELSE IF $event = "DELETE" OR $is_delete  {
                UPSERT type::thing('dbnum_info_table', $ref_0) SET
                    count = count - 1,
                    sesno = $max_sesno,
                    max_ref1 = $ref_1
                WHERE count > 0;
            };
        };
        "#)
        .await?;
    Ok(())
}
