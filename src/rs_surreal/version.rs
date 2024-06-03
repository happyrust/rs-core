use crate::{RefU64, SUL_DB};

///将数据备份到history tables
pub async fn backup_att_and_pe_to_history_tables(refnos: &[RefU64]) -> anyhow::Result<()>{
    let pe_keys = refnos.iter().map(|x| x.to_pe_key()).collect::<Vec<_>>().join(",");
    let sql = format!("fn::backup_atts_and_pes([{}])", pe_keys);
    // println!("sql is {}", &sql);
    let _response = SUL_DB.query(&sql).await.unwrap();
    // dbg!(&response);
    Ok(())
}