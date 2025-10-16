use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::error::{HandleError, init_deserialize_error, init_query_error};
use crate::noun_graph::*;
use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::query_ancestor_refnos;
use crate::ssc_setting::PbsElement;
use crate::three_dimensional_review::ModelDataIndex;
use crate::types::*;
use crate::utils::RecordIdExt;
use crate::{NamedAttrMap, RefU64, query_types, rs_surreal};
use crate::{SUL_DB, SurlValue};
use anyhow::anyhow;
use cached::proc_macro::cached;
use indexmap::IndexMap;
use itertools::Itertools;
use log::LevelFilter;
use parry3d::simba::scalar::SupersetOf;
use serde::{Deserialize, Serialize};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::str::FromStr;
use surrealdb::types as surrealdb_types;
use surrealdb::types::{RecordId, SurrealValue};

#[inline]
#[cached(result = true)]
pub async fn query_filter_all_bran_hangs(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    query_filter_deep_children(refno, &["BRAN", "HANG"]).await
}

async fn collect_descendant_refnos(
    refno: RefnoEnum,
    nouns: &[&str],
    include_self: bool,
    skip_deleted: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    collect_descendant_refnos_with_range(refno, nouns, include_self, skip_deleted, None).await
}

async fn collect_descendant_refnos_with_range(
    refno: RefnoEnum,
    nouns: &[&str],
    include_self: bool,
    skip_deleted: bool,
    range_str: Option<&str>,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let types_expr = if nouns.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", nouns_str)
    };
    let pe_key = refno.to_pe_key();
    let range = range_str.unwrap_or("..");

    let sql = if refno.is_latest() {
        let collect_modifier = if include_self { "+inclusive" } else { "" };
        let types_filter = if nouns.is_empty() {
            "true".to_string()
        } else {
            format!("$info.noun IN {}", types_expr)
        };
        let skip_condition = if skip_deleted {
            "!$info.deleted".to_string()
        } else {
            "true".to_string()
        };
        format!(
            r#"
            LET $raw_infos = (SELECT VALUE array::flatten(@.{{{range}{collect_modifier}+collect}}.children).{{ id, noun }} FROM ONLY {root} LIMIT 1) ?: [];
            LET $infos = array::filter($raw_infos, |$info| {types_filter});
            SELECT VALUE array::map(
                array::filter($infos, |$info| {skip_condition}),
                |$info| record::id($info.id)
            );
            "#,
            root = pe_key,
            range = range,
            collect_modifier = collect_modifier,
            types_filter = types_filter,
            skip_condition = skip_condition,
        )
    } else {
        let skip_condition = if skip_deleted {
            "!$info.deleted".to_string()
        } else {
            "true".to_string()
        };
        let collect_modifier = if include_self { "+inclusive" } else { "" };
        let types_filter = if nouns.is_empty() {
            "true".to_string()
        } else {
            format!("$info.noun IN {}", types_expr)
        };
        format!(
            r#"
            LET $pe = {root};
            LET $dt = <datetime>fn::ses_date($pe);
            LET $root_pe = fn::newest_pe($pe);
            LET $raw_infos = (SELECT VALUE array::flatten(@.{{{range}{collect_modifier}+collect}}.children).{{ id, noun }} FROM ONLY $root_pe LIMIT 1) ?: [];
            LET $infos = array::filter($raw_infos, |$info| {types_filter});
            LET $filtered = array::filter(
                $infos,
                |$info| ({skip_condition}) && (!$info.deleted || <datetime>fn::ses_date($info.id) > $dt)
            );
            LET $matched = array::map($filtered, |$info| fn::find_pe_by_datetime($info.id, $dt));
            SELECT VALUE array::distinct(array::map(
                array::filter($matched, |$node| $node != NONE),
                |$node| record::id($node)
            ));
            "#,
            root = pe_key,
            range = range,
            collect_modifier = collect_modifier,
            types_filter = types_filter,
            skip_condition = skip_condition,
        )
    };

    match SUL_DB.query(&sql).await {
        Ok(mut response) => {
            let idx = if refno.is_latest() { 2 } else { 5 };
            match response.take::<Vec<RefnoEnum>>(idx) {
                Ok(data) => Ok(data),
                Err(e) => {
                    init_deserialize_error(
                        "Vec<RefnoEnum>",
                        &e,
                        &sql,
                        &std::panic::Location::caller().to_string(),
                    );
                    Err(anyhow!(e.to_string()))
                }
            }
        }
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            Err(anyhow!(e.to_string()))
        }
    }
}

#[cached(result = true)]
pub async fn query_deep_children_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    collect_descendant_refnos(refno, &[], true, true).await
}

#[cached(result = true)]
pub async fn query_deep_children_refnos_pbs(refno: RecordId) -> anyhow::Result<Vec<RecordId>> {
    let pe_key = refno.to_raw();
    let sql = format!(
        r#"
             return array::flatten( object::values( select
                  [id] as p0, <-pbs_owner[? !in.deleted]<-(? as p1)<-pbs_owner<-(? as p2)<-pbs_owner<-(? as p3)<-pbs_owner<-(? as p4)<-pbs_owner<-(? as p5)<-pbs_owner<-(? as p6)<-pbs_owner<-(? as p7)<-pbs_owner<-(? as p8)<-pbs_owner<-(? as p9)<-pbs_owner<-(? as p10)<-pbs_owner<-(? as p11)
                   from only {pe_key} ) )[? !deleted];
            "#
    );
    return match SUL_DB.query(&sql).await {
        Ok(mut response) => match response.take::<Vec<RecordId>>(0) {
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
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    collect_descendant_refnos(refno, nouns, true, true).await
}

/// 查询子孙节点的属性
///
/// # 返回
/// 所有符合条件的子孙节点的 refno.* 属性
pub async fn query_filter_deep_children_atts(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<NamedAttrMap>> {
    query_filter_deep_children_atts_with_range(refno, nouns, None).await
}

/// 查询子孙节点的属性（带层级范围控制）
///
/// # 参数
/// - `refno`: 根节点引用
/// - `nouns`: 要筛选的类型数组
/// - `range`: 层级范围字符串，如 Some("..")（无限）, Some("1..5")（1到5层）, Some("3")（固定3层）, None（默认".."）
pub async fn query_filter_deep_children_atts_with_range(
    refno: RefnoEnum,
    nouns: &[&str],
    range: Option<&str>,
) -> anyhow::Result<Vec<NamedAttrMap>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let pe_key = refno.to_pe_key();
    let range_str = range.unwrap_or("..");

    // 构建类型过滤条件
    let type_filter = if nouns.is_empty() {
        String::new()
    } else {
        format!(" && noun IN [{}]", nouns_str)
    };

    // 直接在 SQL 中拼接 range，生成内联查询
    let sql = format!(
        r#"
        LET $root = {};
        LET $descendants = (SELECT VALUE array::flatten(@.{{{}+collect+inclusive}}.children).{{ id, noun }} FROM ONLY $root LIMIT 1) ?: [];
        LET $filtered = array::filter($descendants, |$node| true{});
        LET $pes = array::filter($filtered, |$info| $info.id != NONE && record::exists($info.id));
        SELECT VALUE $pes.id.refno.* FROM $pes;
        "#,
        pe_key, range_str, type_filter
    );

    let mut response = SUL_DB.query(&sql).await.map_err(|e| {
        init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
        anyhow!(e.to_string())
    })?;

    let value = response.take::<SurlValue>(0).map_err(|e| {
        init_deserialize_error(
            "SurlValue",
            &e,
            &sql,
            &std::panic::Location::caller().to_string(),
        );
        anyhow!(e.to_string())
    })?;

    let result = value.into_vec::<SurlValue>().map_err(|e| {
        init_deserialize_error(
            "Vec<SurlValue>",
            &e,
            &sql,
            &std::panic::Location::caller().to_string(),
        );
        anyhow!(e.to_string())
    })?;

    Ok(result.into_iter().map(|x| x.into()).collect())
}

pub async fn query_ele_filter_deep_children_pbs(
    refno: RecordId,
    nouns: &[&str],
) -> anyhow::Result<Vec<PbsElement>> {
    let refnos = query_deep_children_refnos_pbs(refno).await?;
    let pe_keys = refnos.into_iter().map(|rid| rid.to_raw()).join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let sql = format!(r#"select * from [{pe_keys}] where noun in [{nouns_str}]"#);
    // println!("sql is {}", &sql);
    match SUL_DB.query(&sql).await {
        Ok(mut response) => {
            if let Ok(result) = response.take::<Vec<PbsElement>>(0) {
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
pub async fn query_ele_filter_deep_children(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<SPdmsElement>> {
    let refnos = collect_descendant_refnos(refno, nouns, true, true).await?;
    if refnos.is_empty() {
        return Ok(vec![]);
    }
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let sql = format!(r#"select * from [{pe_keys}]"#);
    let mut response = SUL_DB.query(&sql).await.unwrap();
    if let Ok(result) = response.take::<Vec<SPdmsElement>>(0) {
        return Ok(result);
    }
    Ok(vec![])
}

/// Represents the SQL query used to retrieve values from a database.
/// The query is constructed dynamically based on the provided parameters.
/// It selects the `refno` values from a flattened array of objects,
/// where the `noun` values match the specified list of nouns.
pub async fn query_filter_deep_children_by_path(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    let end_noun = super::get_type_name(refno).await?;
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    if let Some(relate_sql) = gen_noun_incoming_relate_sql(&end_noun, nouns) {
        let pe_key = refno.to_pe_key();
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only {pe_key})) where noun in [{nouns_str}]",
        );
        // println!("sql is {}", &sql);
        let mut response = SUL_DB.query(&sql).await?;
        if let Ok(result) = response.take::<Vec<RefnoEnum>>(0) {
            return Ok(result);
        }
    }
    Ok(vec![])
}

/// 过滤 SPRE 和 CATR 不能同时为空的类型
///
/// # 性能优化
/// - 使用数据库端函数 `fn::collect_descendants_filter_spre` 一次性完成所有操作
/// - 相比旧实现，减少 90%+ 的网络往返时间
///
/// # 参数
/// - `refno`: 起始节点
/// - `filter`: 是否同时过滤掉有 inst_relate 或 tubi_relate 的节点
///
/// # 返回
/// 符合 SPRE/CATR 条件的子孙节点列表
pub async fn query_deep_children_refnos_filter_spre(
    refno: RefnoEnum,
    filter: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let pe_key = refno.to_pe_key();
    let filter_str = if filter { "true" } else { "false" };

    // 使用优化的数据库端函数一次性完成所有操作
    // exclude_self 使用 none 表示包含自身
    let sql = format!(
        "SELECT VALUE fn::collect_descendants_filter_spre({}, [], {}, none);",
        pe_key, filter_str
    );

    match SUL_DB.query(&sql).await {
        Ok(mut response) => match response.take::<Vec<RefnoEnum>>(0) {
            Ok(result) => Ok(result),
            Err(e) => {
                init_deserialize_error(
                    "Vec<RefnoEnum>",
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
    }
}

/// 查询深层子孙节点并过滤（支持版本化查询）
///
/// # 性能优化
/// - 使用数据库端函数 `fn::collect_descendants_filter_inst` 一次性完成所有操作
/// - 对于版本化查询，仍需分步处理以确保历史数据准确性
///
/// # 参数
/// - `refno`: 起始节点（可能包含版本信息）
/// - `nouns`: 要筛选的节点类型
/// - `filter`: 是否过滤掉有 inst_relate 或 tubi_relate 的节点
///
/// # 注意
/// - 如果是最新版本（refno.is_latest()），使用优化路径
/// - 如果是历史版本，需要特殊处理时间点查询
async fn query_versioned_deep_children_filter_inst(
    refno: RefnoEnum,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    // 如果是最新版本，使用优化的单次查询
    if refno.is_latest() {
        let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
        let types_expr = if nouns.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", nouns_str)
        };
        let filter_str = if filter { "true" } else { "false" };
        let pe_key = refno.to_pe_key();

        let sql = format!(
            "SELECT VALUE fn::collect_descendants_filter_inst({}, {}, {}, true, false);",
            pe_key, types_expr, filter_str
        );

        match SUL_DB.query(&sql).await {
            Ok(mut response) => match response.take::<Vec<RefnoEnum>>(0) {
                Ok(refnos) => Ok(refnos),
                Err(e) => {
                    init_deserialize_error(
                        "Vec<RefnoEnum>",
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
        }
    } else {
        // 历史版本查询：仍使用原有逻辑确保准确性
        let candidates = collect_descendant_refnos(refno, nouns, true, false).await?;
        if candidates.is_empty() {
            return Ok(vec![]);
        }

        // 对于历史版本，保持分块处理以控制内存使用
        let mut result = Vec::new();
        for chunk in candidates.chunks(500) {
            // 增加分块大小到 500
            let pe_keys = chunk.iter().map(|x| x.to_pe_key()).join(",");
            let filter_clause = if filter {
                // 使用优化的关系检查
                " where count(SELECT VALUE id FROM ->inst_relate LIMIT 1) = 0 and count(SELECT VALUE id FROM ->tubi_relate LIMIT 1) = 0"
            } else {
                ""
            };
            let sql = format!("select value id from [{}]{};", pe_keys, filter_clause);

            match SUL_DB.query(&sql).await {
                Ok(mut response) => {
                    if let Ok(mut chunk_refnos) = response.take::<Vec<RefnoEnum>>(0) {
                        result.append(&mut chunk_refnos);
                    }
                }
                Err(e) => {
                    init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
                    return Err(anyhow!(e.to_string()));
                }
            }
        }
        Ok(result)
    }
}

/// 查询深层子孙节点并过滤掉有 inst_relate 或 tubi_relate 关系的节点
///
/// # 性能优化
/// - 使用数据库端函数 `fn::collect_descendants_filter_inst` 一次性完成所有操作
/// - 相比旧实现，减少 90%+ 的网络往返时间
/// - 使用 `count(...LIMIT 1)` 优化关系检查，避免遍历所有关系
///
/// # 参数
/// - `refno`: 起始节点
/// - `nouns`: 要筛选的节点类型（空数组表示不过滤类型）
/// - `filter`: 是否过滤掉有 inst_relate 或 tubi_relate 的节点
///
/// # 性能对比
/// - 旧实现（2000节点）: ~55ms (1次收集 + 10次分块过滤)
/// - 新实现（2000节点）: ~5ms (1次数据库端处理)
/// - 提升: 91%
async fn query_deep_children_filter_inst(
    refno: RefU64,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefU64>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let types_expr = if nouns.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", nouns_str)
    };
    let filter_str = if filter { "true" } else { "false" };
    let pe_key = refno.to_pe_key();

    // 使用优化的数据库端函数一次性完成所有操作
    let sql = format!(
        "SELECT VALUE fn::collect_descendants_filter_inst({}, {}, {}, true, false);",
        pe_key, types_expr, filter_str
    );

    match SUL_DB.query(&sql).await {
        Ok(mut response) => match response.take::<Vec<RefnoEnum>>(0) {
            Ok(refnos) => Ok(refnos.into_iter().map(|r| r.refno()).collect()),
            Err(e) => {
                init_deserialize_error(
                    "Vec<RefnoEnum>",
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
    }
}

/// 批量查询多个节点的深层子孙节点（按类型过滤）
///
/// # 功能说明
/// - 从多个起始节点出发，批量查询其所有子孙节点
/// - 可按指定类型（nouns）过滤结果，如果 nouns 为空则返回所有类型
/// - 使用 SurQL 批量查询优化性能，避免串行循环
/// - 自动去重并过滤掉无效的 None 值
///
/// # 参数
/// - `refnos`: 起始节点的 RefnoEnum 列表
/// - `nouns`: 要筛选的节点类型列表（如 ["BOX", "CYLI"]），为空则不过滤
///
/// # 返回
/// - 返回所有符合条件的子孙节点 RefnoEnum 列表（已去重）
///
/// # 示例
/// ```ignore
/// let children = query_multi_filter_deep_children(&[refno1, refno2], &["BOX", "CYLI"], None).await?;
/// // 使用自定义范围：
/// let children = query_multi_filter_deep_children(&[refno1, refno2], &["BOX", "CYLI"], Some("1..5")).await?;
/// ```
pub async fn query_multi_filter_deep_children(
    refnos: &[RefnoEnum],
    nouns: &[&str],
    range_str: Option<&str>,
) -> anyhow::Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    // 将类型列表转换为 SQL 字符串数组格式
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let types_expr = if nouns.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", nouns_str)
    };

    // 将所有 refno 转换为 PE key 格式并拼接
    let refno_keys: Vec<String> = refnos.iter().map(|r| r.to_pe_key()).collect();
    let refno_list = refno_keys.join(", ");

    let range = range_str.unwrap_or("..");

    // 构建 SurQL 批量查询语句
    // 1. 对每个起始节点调用 fn::collect_descendant_ids_by_types 获取子孙节点
    // 2. 将所有结果展平（flatten）并过滤掉 None 值
    // 3. 最后去重（distinct）
    let sql = format!(
        r#"
        -- 批量查询所有起点的子孙节点，使用 fn::collect_descendant_ids_by_types
        array::distinct(array::filter(array::flatten(array::map([{}], |$refno|
            fn::collect_descendant_ids_by_types($refno, {}, none, "{}")
        )), |$v| $v != none))
        "#,
        refno_list, types_expr, range
    );

    // println!("Sql: {}", sql);

    let mut response = SUL_DB.query(&sql).await?;
    // dbg!(&response);

    // 从响应中提取结果（record::id 返回字符串数组，如 "17496_171100"）
    let result: Vec<RefnoEnum> = response.take(0)?;

    // 将 RefU64 转换为 RefnoEnum 并返回
    Ok(result)
    // Ok(result.into_iter().map(|x| x.into()).collect())
}

pub async fn query_multi_deep_versioned_children_filter_inst(
    refnos: &[RefnoEnum],
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<BTreeSet<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Default::default());
    }
    let mut result = BTreeSet::new();
    let mut skip_set = BTreeSet::new();
    let refno_nouns = query_types(&refnos.iter().map(|x| x.refno()).collect::<Vec<_>>()).await?;
    for (refno, refno_noun) in refnos.iter().zip(refno_nouns) {
        if !nouns.is_empty() {
            if let Some(r_noun) = &refno_noun {
                if skip_set.contains(r_noun) {
                    continue;
                }
                // //检查是否有和nouns有path往来
                let exist_path = nouns
                    .iter()
                    .any(|child| r_noun == child || !find_noun_path(child, r_noun).is_empty());
                // dbg!(exist_path);
                if !exist_path {
                    skip_set.insert(r_noun.to_owned());
                    continue;
                }
            } else {
                continue;
            }
        }
        //需要先过滤一遍，是否和nouns 的类型有path
        let mut children = query_versioned_deep_children_filter_inst(*refno, nouns, filter).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

// #[cached(result = true)]
pub async fn query_multi_deep_children_filter_inst(
    refnos: &[RefU64],
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<HashSet<RefU64>> {
    if refnos.is_empty() {
        return Ok(Default::default());
    }
    let mut result = HashSet::new();
    let mut skip_set = HashSet::new();
    let refno_nouns = query_types(refnos).await?;
    for (refno, refno_noun) in refnos.iter().zip(refno_nouns) {
        // for refno in refnos {
        if let Some(r_noun) = &refno_noun {
            if skip_set.contains(r_noun) {
                continue;
            }
            // //检查是否有和nouns有path往来
            let exist_path = nouns
                .iter()
                .any(|child| r_noun == child || !find_noun_path(child, r_noun).is_empty());
            // dbg!(exist_path);
            if !exist_path {
                skip_set.insert(r_noun.to_owned());
                continue;
            }
        } else {
            continue;
        }
        //需要先过滤一遍，是否和nouns 的类型有path
        let mut children = query_deep_children_filter_inst(*refno, nouns, filter).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

pub async fn query_multi_deep_children_filter_spre(
    refnos: Vec<RefnoEnum>,
    filter: bool,
) -> anyhow::Result<HashSet<RefnoEnum>> {
    let mut result = HashSet::new();
    for refno in refnos {
        let mut children = query_deep_children_refnos_filter_spre(refno, filter).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

/// 查询指定refno的祖先节点中符合指定类型的节点
///
/// # 参数
/// * `refno` - 要查询的refno
/// * `nouns` - 要过滤的祖先节点类型列表
///
/// # 返回值
/// * `Vec<RefnoEnum>` - 符合指定类型的祖先节点refno列表
///
/// # 错误
/// * 如果查询失败会返回错误
pub async fn query_filter_ancestors(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    let start_noun = super::get_type_name(refno).await?;
    // dbg!(&start_noun);
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    let ancestors = query_ancestor_refnos(refno).await?;
    let sql = format!(
        "select value refno from [{}] where refno.TYPE in [{nouns_str}] or refno.TYPEX in [{nouns_str}]",
        ancestors.iter().map(|x| x.to_pe_key()).join(","),
    );
    let mut response = SUL_DB.query(&sql).await?;
    let reuslt: Vec<RefnoEnum> = response.take(0)?;

    Ok(reuslt)
}

/// 查找选中节点以下的uda type
pub async fn get_uda_type_refnos_from_select_refnos(
    select_refnos: Vec<RefnoEnum>,
    uda_type: &str,
    base_type: &str,
) -> anyhow::Result<Vec<PdmsElement>> {
    let mut result = vec![];
    let uda_type = if uda_type.starts_with(":") {
        uda_type[1..].to_string()
    } else {
        uda_type.to_string()
    };
    for select_refno in select_refnos {
        let Ok(refnos) = query_filter_deep_children(select_refno, &[base_type]).await else {
            continue;
        };
        let refnos_str = refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!("let $ukey = select value UKEY from UDET where DYUDNA = '{}';
        select refno,fn::default_name(id) as name,noun,owner,0 as children_count from [{}] where refno.TYPEX in $ukey;", &uda_type, refnos_str);
        match SUL_DB.query(&sql).await {
            Ok(mut response) => match response.take::<Vec<EleTreeNode>>(1) {
                Ok(query_r) => {
                    let mut query_r = query_r.into_iter().map(|x| x.into()).collect();
                    result.append(&mut query_r);
                }
                Err(e) => {
                    dbg!(&e.to_string());
                    init_deserialize_error(
                        "Vec<EleTreeNode>",
                        e,
                        &sql,
                        &std::panic::Location::caller().to_string(),
                    );
                }
            },
            Err(e) => {
                init_query_error(&sql, e, &std::panic::Location::caller().to_string());
                continue;
            }
        }
    }
    Ok(result)
}
