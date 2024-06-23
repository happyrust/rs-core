use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::error::{init_deserialize_error, init_query_error, HandleError};
use crate::noun_graph::*;
use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{rs_surreal, NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use anyhow::anyhow;
use cached::proc_macro::cached;
use indexmap::IndexMap;
use itertools::Itertools;
use parry3d::simba::scalar::SupersetOf;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::str::FromStr;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TerminalMode, TermLogger, WriteLogger};
use surrealdb::method::Stats;
use surrealdb::sql::Thing;
use crate::ssc_setting::PbsElement;

#[inline]
#[cached(result = true)]
pub async fn query_filter_all_bran_hangs(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    query_filter_deep_children(refno, vec!["BRAN".into(), "HANG".into()]).await
}

#[cached(result = true)]
pub async fn query_deep_children_refnos(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let pe_key = refno.to_pe_key();
    let sql = format!(
        r#"
             return array::flatten( object::values( select
                  [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)<-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)<-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only {pe_key} ) );
            "#
    );
    return match SUL_DB.query(&sql).await {
        Ok(mut response) => match response.take::<Vec<RefU64>>(0) {
            Ok(data) => Ok(data),
            Err(e) => {
                init_deserialize_error(
                    "Vec<RefU64>",
                    &e,
                    &sql,
                    &std::panic::Location::caller().to_string(),
                );
                Err(anyhow!(e.to_string()))
            }
        },
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            Err(anyhow!(e.to_string()))
        }
    };
}

#[cached(result = true)]
pub async fn query_deep_children_refnos_pbs(refno: Thing) -> anyhow::Result<Vec<Thing>> {
    let pe_key = refno.to_string();
    let sql = format!(
        r#"
             return array::flatten( object::values( select
                  [id] as p0, <-pbs_owner<-(? as p1)<-pbs_owner<-(? as p2)<-pbs_owner<-(? as p3)<-pbs_owner<-(? as p4)<-pbs_owner<-(? as p5)<-pbs_owner<-(? as p6)<-pbs_owner<-(? as p7)<-pbs_owner<-(? as p8)<-pbs_owner<-(? as p9)<-pbs_owner<-(? as p10)<-pbs_owner<-(? as p11)
                   from only {pe_key} ) );
            "#
    );
    return match SUL_DB.query(&sql).await {
        Ok(mut response) => match response.take::<Vec<Thing>>(0) {
            Ok(data) => Ok(data),
            Err(e) => {
                init_deserialize_error(
                    "Vec<RefU64>",
                    &e,
                    &sql,
                    &std::panic::Location::caller().to_string(),
                );
                Err(anyhow!(e.to_string()))
            }
        },
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            Err(anyhow!(e.to_string()))
        }
    };
}

pub async fn query_filter_deep_children(
    refno: RefU64,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<RefU64>> {
    let refnos = query_deep_children_refnos(refno).await?;
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let sql = format!(r#"select value id from [{pe_keys}] where noun in [{nouns_str}]"#);
    // println!("sql is {}", &sql);
    match SUL_DB.query(&sql).with_stats().await {
        Ok(mut response) => {
            if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
                return Ok(result);
            }
        }
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            return Err(anyhow!(e.to_string()));
        }
    }
    Ok(vec![])
}

pub async fn query_ele_filter_deep_children_pbs(
    refno: Thing,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<PbsElement>> {
    let refnos = query_deep_children_refnos_pbs(refno).await?;
    let pe_keys = refnos.into_iter().join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let sql = format!(r#"select * from [{pe_keys}] where noun in [{nouns_str}]"#);
    // println!("sql is {}", &sql);
    match SUL_DB.query(&sql).with_stats().await {
        Ok(mut response) => {
            if let Some((stats, Ok(result))) = response.take::<Vec<PbsElement>>(0) {
                return Ok(result);
            }
        }
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            return Err(anyhow!(e.to_string()));
        }
    }
    Ok(vec![])
}

///深度查询
#[cached(result = true)]
pub async fn query_ele_filter_deep_children(
    refno: RefU64,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<SPdmsElement>> {
    let refnos = query_deep_children_refnos(refno).await?;
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let sql = format!(r#"select * from [{pe_keys}] where noun in [{nouns_str}]"#);
    // println!("sql is {}", &sql);
    let mut response = SUL_DB.query(&sql).with_stats().await.unwrap();
    if let Some((stats, Ok(result))) = response.take::<Vec<SPdmsElement>>(0) {
        return Ok(result);
    }
    Ok(vec![])
}

/// Represents the SQL query used to retrieve values from a database.
/// The query is constructed dynamically based on the provided parameters.
/// It selects the `refno` values from a flattened array of objects,
/// where the `noun` values match the specified list of nouns.
#[cached(result = true)]
pub async fn query_filter_deep_children_by_path(
    refno: RefU64,
    nouns: Vec<String>,
) -> anyhow::Result<Vec<RefU64>> {
    let end_noun = super::get_type_name(refno).await?;
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let nouns_slice = nouns.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(relate_sql) = gen_noun_incoming_relate_sql(&end_noun, &nouns_slice) {
        let pe_key = refno.to_pe_key();
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only {pe_key})) where noun in [{nouns_str}]",
        );
        // println!("sql is {}", &sql);
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            return Ok(result);
        }
    }
    Ok(vec![])
}

// #[cached(result = true)]
pub async fn query_deep_children_filter_inst(
    refno: RefU64,
    nouns: Vec<String>,
    filter: bool,
) -> anyhow::Result<Vec<RefU64>> {
    let end_noun = super::get_type_name(refno).await?;
    let nouns_str = rs_surreal::convert_to_sql_str_array(&nouns);
    let pe_key = refno.to_pe_key();
    let mut sql = format!(
        r#"
            let $a = array::flatten( object::values( select
                  [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)<-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)<-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only {pe_key} ) );

            select value refno from $a where noun in [{nouns_str}]"#,
    );
    if filter {
        sql.push_str(" and array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0");
    }
    // println!("query_deep_children_filter_inst sql is: {}", &sql);
    let mut response = SUL_DB.query(&sql).await?;
    // dbg!(&response);
    let result: Vec<RefU64> = response.take(1)?;
    Ok(result)
}

// #[cached(result = true)]
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

// #[cached(result = true)]
pub async fn query_multi_deep_children_filter_inst(
    refnos: Vec<RefU64>,
    nouns: Vec<String>,
    filter: bool,
) -> anyhow::Result<HashSet<RefU64>> {
    let mut result = HashSet::new();
    for refno in refnos {
        let mut children = query_deep_children_filter_inst(refno, nouns.clone(), filter).await?;
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
    if let Some(relate_sql) = gen_noun_outcoming_relate_sql(&start_noun, &nouns_slice) {
        let pe_key = refno.to_pe_key();
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only {pe_key})) where noun in [{nouns_str}]",
        );
        let mut response = SUL_DB.query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefU64>>(0) {
            let execution_time = stats.execution_time;
            return Ok(result);
        }
    }
    Ok(vec![])
}

#[tokio::test]
async fn test_query_filter_deep_children() -> anyhow::Result<()> {
    // 配置日志文件
    let log_file = OpenOptions::new().create(true).append(true).open("error.log")?;
    // 初始化日志系统
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Error, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Error, Config::default(), log_file),
        ]
    ).unwrap();
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    let refno = RefU64::from_str("24383/73927").unwrap();
    let equis = query_filter_deep_children(refno, vec!["EQUI".to_string()]).await?;
    dbg!(&equis.len());
    Ok(())
}
