use glam::Vec3;

use crate::{RefU64, SUL_DB, query_neareast_along_axis};

/// Create the relations between the valves and the floors
pub async fn create_valve_floor_relations() -> anyhow::Result<()> {
    let page_count = 1000;
    let mut offset = 0;
    loop {
        //需要过滤
        //为了测试，暂时只取两个 db 1112 7999
        //where REFNO.dbnum in [1112, 7999] 
        let sql = format!(
            "select value id from VALV start {} limit {page_count}",
            offset
        );
        let mut response = SUL_DB.query(&sql).await?;
        let refnos: Vec<RefU64> = response.take(0).unwrap();
        dbg!(refnos.len());
        if refnos.is_empty() {
            break;
        }
        let mut valve_floor_relations = vec![];
        for refno in refnos {
            let nearest = query_neareast_along_axis(refno, Vec3::NEG_Z, "FLOOR")
                .await
                .unwrap();
            if nearest.is_unset() {
                continue;
            }
            dbg!(nearest);
            valve_floor_relations.push(format!(
                r#"relate pe:{}->spatial_axis_belong->pe:{} set dir="-Z""#,
                refno, nearest
            ));
        } 
        //保存到 SUL_DB
        SUL_DB.query(valve_floor_relations.join(";")).await?;
        offset += page_count;
    }

    Ok(())
}
