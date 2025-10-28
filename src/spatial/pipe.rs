use glam::Vec3;

use crate::{RefU64, RefnoEnum, SUL_DB, SurrealQueryExt, query_neareast_along_axis};

/// Create the relations between the valves and the floors
pub async fn cal_valve_nearest_floor() -> anyhow::Result<()> {
    let page_count = 1000;
    let mut offset = 0;
    loop {
        //需要过滤
        //为了测试，暂时只取两个 db 1112 7999
        //where REFNO.dbnum in [1112, 7999]
        let sql = format!(
            "select value REFNO from VALV where array::len(REFNO->nearest_relate)=0 start {} limit {page_count}",
            offset
        );
        let mut response = SUL_DB.query_response(&sql).await?;
        let refnos: Vec<crate::RefnoEnum> = response.take(0).unwrap();
        if refnos.is_empty() {
            break;
        }
        // dbg!(refnos.len());
        let mut sqls = vec![];
        for refno in refnos {
            if let Ok(Some((nearest, dist))) =
                query_neareast_along_axis(refno, Vec3::NEG_Z, "FLOOR").await
            {
                // 使用 pe 表的主键，避免生成无效的 RecordId
                sqls.push(format!(
                    "relate {}->nearest_relate->{} set dist={dist};",
                    refno.to_pe_key(),
                    nearest.to_pe_key()
                ));
            }
        }
        //保存到 SUL_DB
        if !sqls.is_empty() {
            let batch_sql = sqls.join("");
            SUL_DB.query_response(&batch_sql).await?;
        }
        offset += page_count;
    }

    Ok(())
}
