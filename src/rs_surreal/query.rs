//! 查询模块 - 提供数据库查询功能
//!
//! 这个模块包含了所有与 SurrealDB 数据库交互的查询函数。
//! 主要功能包括：
//! - 基础元素查询
//! - 层次结构查询
//! - 属性数据查询
//! - 历史数据查询
//! - 批量操作

use super::query_mdb_db_nums;
use crate::consts::{MAX_INSERT_LENGTH, WORD_HASH};
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::ssc_setting::PbsElement;
use crate::table::ToTable;
use crate::tool::db_tool::db1_dehash;
use crate::tool::math_tool::*;
use crate::utils::{take_option, take_single, take_vec};
use crate::{DBType, get_db_option, to_table_keys};
use crate::{NamedAttrMap, RefU64};
use crate::{SUL_DB, SurlValue, SurrealQueryExt};
use crate::{graph::QUERY_DEEP_CHILDREN_REFNOS, types::*};
use cached::Cached;
use cached::proc_macro::cached;
use chrono::NaiveDateTime;
use dashmap::DashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use surrealdb::IndexedResults as Response;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::types as surrealdb_types;
use surrealdb::types::{Datetime, SurrealValue, Value};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SurrealValue)]
struct KV<K: SurrealValue, V: SurrealValue> {
    k: K,
    v: V,
}

/// CataHash 分组查询结果
/// k 是一个元组：(cata_hash, exist_inst, ptset)
/// v 是分组的 refnos
#[derive(Clone, Debug, Serialize, Deserialize, SurrealValue)]
pub struct CataHashGroupQueryResult {
    pub k: (String, bool, Option<Vec<CateAxisParam>>),
    pub v: Vec<RefnoEnum>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SurrealValue)]
pub struct RefnoDatetime {
    pub refno: RefnoEnum,
    pub dt: Datetime,
}

/// 按 NOUN 类型查询的层次结构数据项
#[derive(Clone, Debug, Serialize, Deserialize, SurrealValue)]
pub struct NounHierarchyItem {
    /// 元素名称
    pub name: String,
    /// 元素 ID（REFNO）
    pub id: RefnoEnum,
    /// 元素类型（NOUN）
    pub noun: String,
    /// 所有者名称
    pub owner_name: Option<String>,
    /// 所有者参考号
    pub owner: RefnoEnum,
    /// 最后修改日期（通过 fn::ses_date(id) 获取）
    pub last_modified_date: Option<Datetime>,
    /// 直接子节点数量
    pub children_cnt: Option<i32>,
}

/// 通过surql查询PE（Plant Element）数据
///
/// 根据参考号查询对应的工厂元素数据，结果会被缓存以提高性能。
///
/// # 参数
/// * `refno` - 要查询的参考号
///
/// # 返回值
/// * `Result<Option<SPdmsElement>>` - 成功时返回可选的工厂元素数据
///
/// # 错误
/// 如果查询失败，返回错误信息
#[cached(result = true)]
pub async fn get_pe(refno: RefnoEnum) -> anyhow::Result<Option<SPdmsElement>> {
    let sql = format!(
        r#"select * omit id from only {} limit 1;"#,
        refno.to_pe_key()
    );
    SUL_DB.query_take::<Option<SPdmsElement>>(&sql, 0).await
}

/// 获取元素的默认名称
///
/// 查询指定参考号对应的默认名称。
///
/// # 参数
/// * `refno` - 要查询的参考号
///
/// # 返回值
/// * `Result<Option<String>>` - 成功时返回可选的名称字符串
///
/// # 错误
/// 如果查询失败，返回错误信息
pub async fn get_default_name(refno: RefnoEnum) -> anyhow::Result<Option<String>> {
    let sql = format!("return fn::default_name({});", refno.to_pe_key());
    SUL_DB.query_take::<Option<String>>(&sql, 0).await
}

///查询到祖先节点列表
/// 获取指定refno的所有祖先节点
///
/// # 参数
/// * `refno` - 要查询的refno
///
/// # 返回值
/// * `Vec<RefnoEnum>` - 祖先节点的refno列表
///
/// # 错误
/// * 如果查询失败会返回错误
#[cached(result = true)]
pub async fn query_ancestor_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!("return fn::ancestor({}).refno;", refno.to_pe_key());
    SUL_DB.query_take::<Vec<RefnoEnum>>(&sql, 0).await
}

/// 查询指定类型的第一个祖先节点
///
/// # 参数
/// * `refno` - 要查询的refno
/// * `ancestor_type` - 要查询的祖先节点类型
///
/// # 返回值
/// * `Option<RefnoEnum>` - 如果找到则返回对应的祖先节点refno,否则返回None
///
/// # 错误
/// * 如果查询失败会返回错误
#[cached(
    result = true,
    key = "(RefnoEnum, String)",
    convert = r#"{ (refno, ancestor_type.to_string()) }"#
)]
pub async fn query_ancestor_refno_by_type(
    refno: RefnoEnum,
    ancestor_type: &str,
) -> anyhow::Result<Option<RefnoEnum>> {
    let sql = format!(
        "return fn::ancestor({})[where noun='{}'][0]?.refno;",
        refno.to_pe_key(),
        ancestor_type
    );
    SUL_DB.query_take::<Option<RefnoEnum>>(&sql, 0).await
}

// #[cached(result = true)]
/// 通过元素名称查询参考号
///
/// 根据元素的完整路径名称查询对应的参考号。
///
/// # 参数
/// * `name` - 要查询的元素完整路径名称（以'/'开头）
///
/// # 返回值
/// * `Result<Option<RefnoEnum>>` - 成功时返回可选的参考号
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let refno = get_refno_by_name("/plant/equipment/pump1").await?;
/// ```
pub async fn get_refno_by_name(name: &str) -> anyhow::Result<Option<RefnoEnum>> {
    let sql = format!(
        r#"select value id from only pe where name="/{}" limit 1;"#,
        name
    );
    println!("sql is {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let s = response.take::<Option<RefnoEnum>>(0);
    Ok(s?)
}

/// 获取指定refno的所有祖先节点的类型名称
///
/// # 参数
/// * `refno` - 要查询的refno
///
/// # 返回值
/// * `Vec<String>` - 祖先节点的类型名称列表
///
/// # 错误
/// * 如果查询失败会返回错误
#[cached(result = true)]
pub async fn get_ancestor_types(refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
    let sql = format!("return fn::ancestor({}).noun;", refno.to_pe_key());
    SUL_DB.query_take::<Vec<String>>(&sql, 0).await
}

///查询到祖先节点属性数据
/// 查询指定refno的所有祖先节点的属性数据
///
/// # 参数
/// * `refno` - 要查询的refno
///
/// # 返回值
/// * `Vec<NamedAttrMap>` - 祖先节点的属性数据列表,包含每个节点的名称和属性映射
///
/// # 错误
/// * 如果查询失败会返回错误
pub async fn get_ancestor_attmaps(refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
    let sql = format!("return fn::ancestor({}).refno.*;", refno.to_pe_key());
    let raw_values: Vec<SurlValue> = SUL_DB.query_take(&sql, 0).await?;
    // 过滤掉 NONE 值
    let named_attmaps: Vec<NamedAttrMap> = raw_values
        .into_iter()
        .filter_map(|x| {
            let val: Result<NamedAttrMap, _> = x.try_into();
            val.ok() // 将 Err 转换为 None，从而过滤掉无法转换的值
        })
        .collect();
    Ok(named_attmaps)
}

/// 获取指定refno的类型名称
///
/// # 参数
/// * `refno` - 要查询的refno
///
/// # 返回值
/// * `String` - 类型名称，如果未找到则返回"unset"
#[cached(result = true)]
pub async fn get_type_name(refno: RefnoEnum) -> anyhow::Result<String> {
    let sql = format!("select value noun from only {} limit 1", refno.to_pe_key());
    let type_name: Option<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(type_name.unwrap_or("unset".to_owned()))
}

/// 批量获取多个参考号的类型名称
///
/// 根据提供的参考号迭代器，批量查询每个参考号对应的类型名称。
///
/// # 参数
/// * `refnos` - 参考号迭代器，包含要查询的参考号列表
///
/// # 返回值
/// * `Result<Vec<String>>` - 成功时返回类型名称列表，与输入参考号顺序一致
///
/// # 错误
/// 如果查询过程中发生错误，返回错误信息
///
/// # 示例
/// ```
/// let refnos = vec![refno1, refno2, refno3];
/// let type_names = get_type_names(refnos.iter()).await?;
/// ```
pub async fn get_type_names(
    refnos: impl Iterator<Item = &RefnoEnum>,
) -> anyhow::Result<Vec<String>> {
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let sql = format!(r#"select value noun from [{}]"#, pe_keys);
    let type_names: Vec<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(type_names)
}

/// 获取拥有者类型名称
///
/// 根据参考号查询其拥有者（owner）的类型名称。
///
/// # 参数
/// * `refno` - 要查询的参考号
///
/// # 返回值
/// * `Result<String>` - 成功时返回拥有者的类型名称
///
/// # 错误
/// 如果查询失败或找不到拥有者，返回错误信息
///
/// # 注意
/// 此函数会查询参考号对应的拥有者，然后返回拥有者的类型名称。
/// 如果参考号没有拥有者或查询失败，将返回错误。
pub async fn get_owner_type_name(refno: RefU64) -> anyhow::Result<String> {
    let sql = format!(
        "return (select value owner.noun from only (type::record('pe', {})));",
        refno.to_pe_key()
    );
    let owner_type: Option<String> = SUL_DB.query_take(&sql, 0).await?;
    owner_type.ok_or_else(|| anyhow::anyhow!("Owner not found for refno: {}", refno))
}
/// 获取元素自身及其拥有者的类型名称
///
/// 查询指定参考号对应的元素类型名称及其直接拥有者的类型名称。
/// 结果会被缓存以提高性能。
///
/// # 参数
/// * `refno` - 要查询的参考号
///
/// # 返回值
/// * `Result<Vec<String>>` - 包含两个元素的向量：
///   - 第一个元素是元素自身的类型名称
///   - 第二个元素是拥有者的类型名称（如果没有拥有者则为空字符串）
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let types = get_type_and_owner_type(refno).await?;
/// let self_type = &types[0];  // 元素自身类型
/// let owner_type = &types[1]; // 拥有者类型
/// ```
#[cached(result = true)]
pub async fn get_type_and_owner_type(refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
    let sql = format!(
        "select value [noun, owner.noun] from only {} limit 1",
        refno.to_pe_key()
    );
    SUL_DB.query_take::<Vec<String>>(&sql, 0).await
}

/// 判断元素的拥有者是否为指定类型
///
/// 查询指定参考号的拥有者类型，并判断是否匹配给定的类型。
/// 结果会被缓存以提高性能。
///
/// # 参数
/// * `refno` - 要查询的参考号
/// * `owner_type` - 要检查的拥有者类型
///
/// # 返回值
/// * `Result<bool>` - 如果拥有者类型匹配则返回 true，否则返回 false
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let is_pipe_owner = is_owner_type(refno, "PIPE").await?;
/// if is_pipe_owner {
///     println!("该元素的拥有者是管道类型");
/// }
/// ```
#[cached(
    result = true,
    key = "(RefnoEnum, String)",
    convert = r#"{ (refno, owner_type.to_string()) }"#
)]
pub async fn is_owner_type(refno: RefnoEnum, owner_type: &str) -> anyhow::Result<bool> {
    let sql = format!(
        "select value owner.noun from only {} limit 1",
        refno.to_pe_key()
    );
    let actual_owner_type: Option<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(actual_owner_type.as_deref() == Some(owner_type))
}

/// 判断元素的拥有者是否在指定类型列表中
///
/// 查询指定参考号的拥有者类型，并判断是否在给定的类型列表中。
/// 结果会被缓存以提高性能。
///
/// # 参数
/// * `refno` - 要查询的参考号
/// * `owner_types` - 要检查的拥有者类型切片
///
/// # 返回值
/// * `Result<bool>` - 如果拥有者类型在列表中则返回 true，否则返回 false
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let types = ["PIPE", "EQUIPMENT", "VALVE"];
/// let is_special_owner = is_owner_type_in(refno, &types).await?;
/// if is_special_owner {
///     println!("该元素的拥有者是特殊类型");
/// }
/// ```
#[cached(
    result = true,
    key = "(RefnoEnum, String)",
    convert = r#"{ (refno, format!("{:?}", owner_types)) }"#
)]
pub async fn is_owner_type_in(refno: RefnoEnum, owner_types: &[&str]) -> anyhow::Result<bool> {
    if owner_types.is_empty() {
        return Ok(false);
    }
    let types_str = owner_types.iter().map(|t| format!("'{}'", t)).join(",");
    let sql = format!(
        "select value owner.noun from only {} where owner.noun IN ({}) limit 1",
        refno.to_pe_key(),
        types_str
    );
    let actual_owner_type: Option<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(actual_owner_type.is_some())
}

/// 获取指定类型的拥有者参考号
///
/// 查询指定参考号的拥有者参考号，并确保拥有者是指定类型。
/// 结果会被缓存以提高性能。
///
/// # 参数
/// * `refno` - 要查询的参考号
/// * `owner_type` - 要求的拥有者类型
///
/// # 返回值
/// * `Result<Option<RefnoEnum>>` - 如果拥有者存在且类型匹配则返回拥有者参考号，否则返回 None
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let pipe_owner_refno = get_owner_refno_by_type(refno, "PIPE").await?;
/// if let Some(owner_refno) = pipe_owner_refno {
///     println!("找到管道类型的拥有者: {:?}", owner_refno);
/// }
/// ```
#[cached(
    result = true,
    key = "(RefnoEnum, String)",
    convert = r#"{ (refno, owner_type.to_string()) }"#
)]
pub async fn get_owner_refno_by_type(refno: RefnoEnum, owner_type: &str) -> anyhow::Result<Option<RefnoEnum>> {
    let sql = format!(
        "select value owner from only {} where owner.noun = '{}' limit 1",
        refno.to_pe_key(),
        owner_type
    );
    SUL_DB.query_take::<Option<RefnoEnum>>(&sql, 0).await
}

/// 获取指定类型列表中的拥有者参考号
///
/// 查询指定参考号的拥有者参考号，并确保拥有者是在指定的类型列表中。
/// 结果会被缓存以提高性能。
///
/// # 参数
/// * `refno` - 要查询的参考号
/// * `owner_types` - 要求的拥有者类型切片
///
/// # 返回值
/// * `Result<Option<RefnoEnum>>` - 如果拥有者存在且类型在列表中则返回拥有者参考号，否则返回 None
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let types = ["PIPE", "EQUIPMENT", "VALVE"];
/// let special_owner_refno = get_owner_refno_by_types(refno, &types).await?;
/// if let Some(owner_refno) = special_owner_refno {
///     println!("找到特殊类型的拥有者: {:?}", owner_refno);
/// }
/// ```
#[cached(
    result = true,
    key = "(RefnoEnum, String)",
    convert = r#"{ (refno, format!("{:?}", owner_types)) }"#
)]
pub async fn get_owner_refno_by_types(refno: RefnoEnum, owner_types: &[&str]) -> anyhow::Result<Option<RefnoEnum>> {
    if owner_types.is_empty() {
        return Ok(None);
    }
    let types_str = owner_types.iter().map(|t| format!("'{}'", t)).join(",");
    let sql = format!(
        "select value owner from only {} where owner.noun IN ({}) limit 1",
        refno.to_pe_key(),
        types_str
    );
    SUL_DB.query_take::<Option<RefnoEnum>>(&sql, 0).await
}

/// 批量获取指定类型的拥有者参考号
///
/// 批量查询多个参考号的拥有者参考号，并过滤出拥有者是指定类型的项。
///
/// # 参数
/// * `refnos` - 要查询的参考号迭代器
/// * `owner_type` - 要求的拥有者类型
///
/// # 返回值
/// * `Result<Vec<Option<RefnoEnum>>>` - 返回与输入顺序对应的拥有者参考号列表，
///   如果拥有者不存在或类型不匹配则对应位置为 None
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let refnos = vec![refno1, refno2, refno3];
/// let pipe_owners = get_owner_refnos_by_type(refnos.iter(), "PIPE").await?;
/// for (i, owner_refno) in pipe_owners.iter().enumerate() {
///     if let Some(owner) = owner_refno {
///         println!("refno {:?} 的管道拥有者: {:?}", refnos[i], owner);
///     }
/// }
/// ```
pub async fn get_owner_refnos_by_type(
    refnos: impl Iterator<Item = &RefnoEnum>,
    owner_type: &str,
) -> anyhow::Result<Vec<Option<RefnoEnum>>> {
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let sql = format!(
        "select value owner from [{}] where owner.noun = '{}'",
        pe_keys,
        owner_type
    );
    SUL_DB.query_take::<Vec<Option<RefnoEnum>>>(&sql, 0).await
}

/// 批量获取指定类型列表中的拥有者参考号
///
/// 批量查询多个参考号的拥有者参考号，并过滤出拥有者是在指定类型列表中的项。
///
/// # 参数
/// * `refnos` - 要查询的参考号迭代器
/// * `owner_types` - 要求的拥有者类型切片
///
/// # 返回值
/// * `Result<Vec<Option<RefnoEnum>>>` - 返回与输入顺序对应的拥有者参考号列表，
///   如果拥有者不存在或类型不在列表中则对应位置为 None
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```
/// let refnos = vec![refno1, refno2, refno3];
/// let types = ["PIPE", "EQUIPMENT", "VALVE"];
/// let special_owners = get_owner_refnos_by_types(refnos.iter(), &types).await?;
/// for (i, owner_refno) in special_owners.iter().enumerate() {
///     if let Some(owner) = owner_refno {
///         println!("refno {:?} 的特殊类型拥有者: {:?}", refnos[i], owner);
///     }
/// }
/// ```
pub async fn get_owner_refnos_by_types(
    refnos: impl Iterator<Item = &RefnoEnum>,
    owner_types: &[&str],
) -> anyhow::Result<Vec<Option<RefnoEnum>>> {
    if owner_types.is_empty() {
        return Ok(vec![]);
    }
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let types_str = owner_types.iter().map(|t| format!("'{}'", t)).join(",");
    let sql = format!(
        "array::distinct(select value owner from [{}] where owner.noun IN [{}])",
        pe_keys,
        types_str
    );
    println!("DEBUG: get_owner_refnos_by_types SQL: {}", sql);
    SUL_DB.query_take::<Vec<Option<RefnoEnum>>>(&sql, 0).await
}

/// 获取元素在父节点下的索引位置
///
/// 根据元素在父节点下的位置返回其索引值，可选按类型过滤。
///
/// # 参数
/// * `parent` - 父节点的参考号
/// * `refno` - 要查询的子节点参考号
/// * `noun` - 可选参数，如果提供则只统计该类型的子节点
///
/// # 返回值
/// * `Result<Option<u32>>` - 成功时返回子节点的索引（从0开始），如果未找到则返回None
///
/// # 错误
/// 如果查询子节点列表或类型名称时出错，返回错误信息
///
/// # 示例
/// ```
/// // 获取在父节点下的所有子节点中的索引
/// let index = get_index_by_noun_in_parent(parent_refno, child_refno, None).await?;
///
/// // 只统计特定类型的子节点中的索引
/// let index = get_index_by_noun_in_parent(parent_refno, child_refno, Some("PIPE")).await?;
/// ```
///
/// 该函数使用 SurrealDB 的图查询功能直接获取元素在父节点下的索引。
/// 通过使用图查询，可以提高性能并减少数据库查询次数。
pub async fn get_index_by_noun_in_parent(
    parent: RefnoEnum,
    refno: RefnoEnum,
    noun: Option<&str>,
) -> anyhow::Result<Option<u32>> {
    let sql = format!(
        r#"
        array::find_index((select value in.id from {}<-pe_owner {}), {})
    "#,
        parent.to_pe_key(),
        if let Some(noun) = noun {
            format!("where in.noun='{}'", noun)
        } else {
            "".to_owned()
        },
        refno.to_pe_key()
    );
    SUL_DB.query_take(&sql, 0).await
}

/// 获取上一个版本的参考号及时间戳
///
/// 查询指定参考号的上一个历史版本的参考号及其对应的时间戳。
///
/// # 参数
/// * `refno_enum` - 当前版本的参考号
///
/// # 返回值
/// * `Result<Option<RefnoDatetime>>` - 成功时返回包含参考号和时间戳的元组，如果没有上一个版本则返回None
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 注意
/// 此函数通过查询`old_pe`字段获取上一个版本的参考号
pub async fn query_prev_dt_refno(refno_enum: RefnoEnum) -> anyhow::Result<Option<RefnoDatetime>> {
    let sql = format!(
        "select old_pe as refno, fn::ses_date(old_pe) as dt from only {} where old_pe!=none limit 1;",
        refno_enum.to_pe_key(),
    );
    // println!("query_prev_version_refno sql is {}", &sql);
    SUL_DB.query_take::<Option<RefnoDatetime>>(&sql, 0).await
}

/// 获取带时间戳的当前版本参考号
///
/// 查询指定参考号及其对应的时间戳信息。
///
/// # 参数
/// * `refno_enum` - 要查询的参考号
///
/// # 返回值
/// * `Result<Option<RefnoDatetime>>` - 成功时返回包含参考号和时间戳的元组
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 注意
/// 此函数通过`fn::ses_date`函数获取参考号对应的时间戳
pub async fn query_dt_refno(refno_enum: RefnoEnum) -> anyhow::Result<Option<RefnoDatetime>> {
    let sql = format!(
        "select id as refno, fn::ses_date(id) as dt from only {} limit 1;",
        refno_enum.to_pe_key(),
    );
    // println!("query_dt_refno sql is {}", &sql);
    SUL_DB.query_take::<Option<RefnoDatetime>>(&sql, 0).await
}

/// 获取上一个版本的UI属性映射
///
/// 查询指定参考号的上一个历史版本的UI属性映射。
///
/// # 参数
/// * `refno_enum` - 当前版本的参考号
///
/// # 返回值
/// * `Result<NamedAttrMap>` - 成功时返回上一个版本的属性映射，如果不存在上一个版本则返回空映射
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 注意
/// 此函数会先通过`query_prev_dt_refno`获取上一个版本的参考号，
/// 然后调用`get_ui_named_attmap`获取该版本的属性映射
pub async fn get_ui_named_attmap_prev_version(
    refno_enum: RefnoEnum,
) -> anyhow::Result<NamedAttrMap> {
    if let Some(refno_datetime) = query_prev_dt_refno(refno_enum).await? {
        return get_ui_named_attmap(refno_datetime.refno).await;
    }
    Ok(NamedAttrMap::default())
}

/// 查询子节点的完整名称映射
///
/// 获取指定父节点下所有子节点的参考号与完整名称的映射关系。
///
/// # 参数
/// * `refno` - 父节点的参考号
///
/// # 返回值
/// * `Result<IndexMap<RefnoEnum, String>>` - 成功时返回子节点参考号到完整名称的映射
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 注意
/// 使用图查询获取所有通过pe_owner关系连接的子节点，并获取它们的完整路径名称
pub async fn query_children_full_names_map(
    refno: RefnoEnum,
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    let mut response = SUL_DB
        .query(format!(
            "select value [in, fn::default_full_name(in)] from {}<-pe_owner where record::exists(in)",
            refno.to_pe_key()
        ))
        .await?;
    let map: Vec<(RefnoEnum, String)> = response.take(0)?;
    let map = IndexMap::from_iter(map);
    Ok(map)
}

/// 批量查询多个参考号的完整名称映射
///
/// 获取多个参考号对应的完整名称映射关系。
///
/// # 参数
/// * `refnos` - 参考号切片
///
/// # 返回值
/// * `Result<IndexMap<RefnoEnum, String>>` - 成功时返回参考号到完整名称的映射
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 注意
/// 此函数会批量查询多个参考号的完整路径名称，适用于需要获取多个元素的完整路径名称的场景
pub async fn query_full_names_map(
    refnos: &[RefnoEnum],
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    let mut response = SUL_DB
        .query(format!(
            "select value [id, fn::default_full_name(id)] from {}",
            refnos
                .iter()
                .map(|x| x.to_pe_key())
                .collect::<Vec<_>>()
                .join(",")
        ))
        .await?;
    let map: Vec<(RefnoEnum, String)> = response.take(0)?;
    let map = IndexMap::from_iter(map);
    Ok(map)
}

/// 批量查询多个参考号的完整名称列表
///
/// 获取多个参考号对应的完整名称列表，保持与输入参考号相同的顺序。
///
/// # 参数
/// * `refnos` - 参考号切片
///
/// # 返回值
/// * `Result<Vec<String>>` - 成功时返回完整名称的列表，与输入参考号顺序一致
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 注意
/// 此函数返回的名称列表顺序与输入参考号的顺序一致
pub async fn query_full_names(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<String>> {
    let sql = format!(
        "select value fn::default_full_name(id) from [{}]",
        refnos.into_iter().map(|x| x.to_pe_key()).join(",")
    );
    let names: Vec<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(names)
}

///查询的数据把 refno->name，换成名称
// #[cached(result = true)]
/// 查询数据并将 refno->name 替换为名称
///
/// # 参数
///
/// * `refno` - 需要查询的 RefnoEnum
///
/// # 返回值
///
/// 返回一个包含 RefnoEnum 和名称的 IndexMap
///
/// # 错误
///
/// 如果查询失败，将返回一个错误
pub async fn query_data_with_refno_to_name(
    refno: RefnoEnum,
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    let sql = format!(
        "select value [in, fn::default_full_name(in)] from {}<-pe_owner where record::exists(in)",
        refno.to_pe_key()
    );
    let map: Vec<(RefnoEnum, String)> = SUL_DB.query_take(&sql, 0).await?;
    let map = IndexMap::from_iter(map);
    Ok(map)
}

/// 查询多个 refno 并将其转换为名称
///
/// # 参数
///
/// * `refnos` - 需要查询的 RefnoEnum 列表
///
/// # 返回值
///
/// 返回一个包含 RefnoEnum 和名称的 IndexMap
///
/// # 错误
///
/// 如果查询失败，将返回一个错误
pub async fn query_multiple_refnos_to_names(
    refnos: &[RefnoEnum],
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    let sql = format!(
        "select value fn::default_full_name(id) from [{}]",
        refnos.into_iter().map(|x| x.to_pe_key()).join(",")
    );
    let names: Vec<String> = SUL_DB.query_take(&sql, 0).await?;
    let map = IndexMap::from_iter(refnos.iter().cloned().zip(names));
    Ok(map)
}

/// 查询多个 refno 并返回其名称列表
///
/// # 参数
///
/// * `refnos` - 需要查询的 RefnoEnum 列表
///
/// # 返回值
///
/// 返回一个包含名称的 Vec
///
/// # 错误
///
/// 如果查询失败，将返回一个错误
pub async fn query_refnos_to_names_list(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<String>> {
    let sql = format!(
        "select value fn::default_full_name(id) from [{}]",
        refnos.into_iter().map(|x| x.to_pe_key()).join(",")
    );
    let names: Vec<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(names)
}

pub async fn get_ui_named_attmap(refno_enum: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    let mut attmap = get_named_attmap_with_uda(refno_enum).await?;
    attmap.fill_explicit_default_values();
    let mut refno_fields: Vec<RefnoEnum> = vec![];
    let mut keys = vec![];
    let mut unset_keys = vec![];
    let mut new_desp = None;
    let mut tuples = vec![];
    let unip = attmap.get_i32_vec("UNIPAR").unwrap_or_default();
    // dbg!(&attmap);
    for (k, v) in &mut attmap.map {
        if k == "REFNO" {
            if let NamedAttrValue::RefnoEnumType(r) = v {
                *v = NamedAttrValue::RefU64Type(r.refno().into());
            }
            continue;
        }
        if k == "UNIPAR" || k == "SESNO" {
            continue;
        }
        match v {
            NamedAttrValue::RefU64Type(r) => {
                if r.is_valid() {
                    refno_fields.push((*r).into());
                    keys.push(k.to_owned());
                } else {
                    unset_keys.push(k.to_owned());
                }
            }
            NamedAttrValue::RefnoEnumType(r) => {
                if r.refno().is_valid() {
                    refno_fields.push(*r);
                    keys.push(k.to_owned());
                } else {
                    unset_keys.push(k.to_owned());
                }
            }
            NamedAttrValue::Vec3Type(d) => {
                if k == "ORI" {
                    tuples.push((
                        k.clone(),
                        NamedAttrValue::StringType(dquat_to_pdms_ori_xyz_str(
                            &angles_to_dori(*d).unwrap_or_default(),
                            false,
                        )),
                    ));
                } else if k.contains("POS") {
                    tuples.push((k.clone(), NamedAttrValue::StringType(vec3_to_xyz_str(*d))));
                } else {
                    //默认是方向
                    tuples.push((
                        k.clone(),
                        NamedAttrValue::StringType(convert_to_xyz(&to_pdms_dvec_str(
                            &d.as_dvec3(),
                            false,
                        ))),
                    ));
                }
            }
            NamedAttrValue::F32VecType(d) => {
                if k == "DESP" {
                    let mut vec = vec![];
                    for (v, n) in d.iter().zip(&unip) {
                        // 检查 UNIPAR 类型：如果是 WORD 类型，需要还原为字符串
                        if *n == WORD_HASH as i32 {
                            vec.push(db1_dehash(*v as u32));
                        } else {
                            // 数值类型直接转换为字符串
                            vec.push(v.to_string());
                        }
                    }
                    new_desp = Some(vec);
                }
            }
            NamedAttrValue::InvalidType => {
                unset_keys.push(k.to_owned());
            }
            _ => {}
        }
    }
    if let Some(new_desp) = new_desp {
        attmap.insert("DESP".to_owned(), NamedAttrValue::StringArrayType(new_desp));
        attmap.remove("UNIPAR");
    }

    for (k, v) in tuples {
        attmap.insert(k, v);
    }

    let names = query_full_names(&refno_fields).await.unwrap_or_default();
    for (k, v) in keys.into_iter().zip(names) {
        attmap.insert(
            k,
            NamedAttrValue::StringType(if v.is_empty() { "unset".to_owned() } else { v }),
        );
    }
    for k in unset_keys {
        attmap.insert(k, NamedAttrValue::StringType("unset".to_owned()));
    }

    attmap.remove("SESNO");
    Ok(attmap)
}

///通过surql查询属性数据
#[cached(result = true)]
pub async fn get_named_attmap(refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    let sql = format!(r#"(select * from {}.refno)[0];"#, refno.to_pe_key());
    let named_attmap: Option<NamedAttrMap> = SUL_DB.query_take(&sql, 0).await?;
    Ok(named_attmap.unwrap_or_default())
}

#[cached(result = true)]
pub async fn get_siblings(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!("select value in from {}<-pe_owner", refno.to_pe_key());
    SUL_DB.query_take::<Vec<RefnoEnum>>(&sql, 0).await
}

#[cached(result = true)]
pub async fn get_next_prev(refno: RefnoEnum, next: bool) -> anyhow::Result<RefnoEnum> {
    let siblings = get_siblings(refno).await?;
    let pos = siblings
        .iter()
        .position(|x| *x == refno)
        .unwrap_or_default();
    if next {
        Ok(siblings.get(pos + 1).cloned().unwrap_or_default())
    } else {
        if pos == 0 {
            return Ok(Default::default());
        }
        Ok(siblings.get(pos - 1).cloned().unwrap_or_default())
    }
}

/// Get the default full name for a pipe element
///
/// Wraps the Surreal function fn::default_full_name
#[cached(result = true)]
pub async fn get_default_full_name(refno: RefnoEnum) -> anyhow::Result<String> {
    let sql = format!("RETURN fn::default_full_name({})", refno.to_pe_key());
    let result: Option<String> = SUL_DB.query_take(&sql, 0).await?;
    Ok(result.unwrap_or_default())
}

/// 通过surql查询属性数据，包含UDA数据
///
/// 这个函数用于获取指定参考号的属性映射，包括其UDA（用户定义属性）数据。
/// 如果结果已被缓存，则直接返回缓存的结果。
///
/// # 参数
///
/// * `refno_enum` - 要查询的参考号
///
/// # 返回值
///
/// 返回一个包含所有属性和UDA的`NamedAttrMap`
///
/// # 错误
///
/// 如果查询失败，返回错误信息
#[cached(result = true)]
pub(crate) async fn get_named_attmap_with_uda(
    refno_enum: RefnoEnum,
) -> anyhow::Result<NamedAttrMap> {
    // 构建SQL查询语句，包含三个主要部分：
    // 1. 查询元素的基本属性和PE（Plant Element）信息
    // 2. 查询默认的UDA（用户定义属性）
    // 3. 查询覆盖的UDA值
    let sql = format!(
        r#"
        -- 1. 通过refno查询元素的完整名称和所有属性
        select fn::default_full_name(REFNO) as NAME, * from only {0}.refno fetch pe;
        
        -- 2. 查询默认的UDA（用户定义属性）
        -- 如果UDNA为空，则使用DYUDNA作为属性名
        select string::concat(':', if UDNA==none || string::len(UDNA)==0 {{ DYUDNA }} else {{ UDNA }}) as u, 
               DFLT as v, 
               UTYP as t 
        from UDA 
        where !UHIDE and {0}.noun in ELEL;
        
        -- 3. 查询覆盖的UDA值
        -- 从ATT_UDA表中获取覆盖的UDA值
        select string::concat(':', if u.UDNA==none || string::len(u.UDNA)==0 {{ u.DYUDNA }} else {{ u.UDNA }}) as u, 
               u.UTYP as t, 
               v 
        from (ATT_UDA:{1}).udas 
        where u.UTYP != none;
        "#,
        refno_enum.to_pe_key(), // 转换为PE键名格式
        refno_enum.refno()      // 获取参考号
    );

    // 定义用于反序列化UDA键值对的结构体
    #[derive(Debug, Deserialize, SurrealValue)]
    struct UdaKv {
        u: String,
        t: Option<String>,
        v: SurlValue,
    }

    // 执行查询并依次处理三个结果集
    let mut response = SUL_DB.query_response(&sql).await?;
    let mut named_attmap = response
        .take::<Option<NamedAttrMap>>(0)?
        .unwrap_or_default();

    let mut apply_uda_entries = |entries: Vec<UdaKv>| {
        for UdaKv { u: uname, t, v } in entries {
            if uname == ":NONE" || uname == ":unset" || uname.is_empty() {
                continue;
            }
            let type_name = t.as_deref().unwrap_or("TEXT");
            let att_value = NamedAttrValue::from((type_name, v));
            named_attmap.insert(uname, att_value);
        }
    };

    apply_uda_entries(response.take(1)?);
    apply_uda_entries(response.take(2)?);

    Ok(named_attmap)
}

pub const CATR_QUERY_STR: &'static str = "refno.CATR.refno.CATR, refno.CATR.refno.PRTREF.refno.CATR, refno.SPRE, refno.SPRE.refno.CATR, refno.CATR";

/// 获取元素的CATR参考号
///
/// 这个函数会尝试通过两种方式获取元素的CATR参考号：
/// 1. 直接查询元素的CATR属性
/// 2. 查询元素的SPRE属性，并从其中获取CATR参考号
#[cached(result = true)]
pub async fn get_cat_refno(refno: RefnoEnum) -> anyhow::Result<Option<RefnoEnum>> {
    // 尝试通过查询属性获取CATR参考号
    if let Ok(spre_map) = query_single_by_paths(refno, &["->SPRE", "->SPRE->CATR"], &[]).await {
        // 从SPRE属性中获取CATR参考号
        if let Some(cat_value) = spre_map.map.get("CATR") {
            // 从获取的CATR值中提取RefnoEnum类型的参考号
            if let Some(cat_refno) = extract_refno_enum(cat_value) {
                return Ok(Some(cat_refno));
            }
        }
    }

    // 尝试通过SQL查询获取CATR参考号
    query_catr_via_sql(refno).await
}

/// 通过SQL查询获取元素的CATR参考号
///
/// 这个函数会查询元素的CATR属性和SPRE属性，并从中获取CATR参考号。
/// 如果查询结果为空，则返回None。
async fn query_catr_via_sql(refno: RefnoEnum) -> anyhow::Result<Option<RefnoEnum>> {
    let catr_sql = format!(
        r#"
        select value array::first(array::flatten([
            refno.CATR.refno.CATR[where noun in ["SCOM", "SPRF", "SFIT", "JOIN", "SPCO"]],
            refno.CATR.refno.PRTREF.refno.CATR[where noun in ["SCOM", "SPRF", "SFIT", "JOIN", "SPCO"]],
            refno.CATR[where noun in ["SCOM", "SPRF", "SFIT", "JOIN", "SPCO"]]
        ]))
        from only {} limit 1;
    "#,
        refno.to_pe_key()
    );

    SUL_DB.query_take::<Option<RefnoEnum>>(&catr_sql, 0).await
}

/// 从命名属性值中提取RefnoEnum类型的参考号
///
/// 这个函数会尝试从命名属性值中提取RefnoEnum类型的参考号，
/// 如果提取失败，则返回None。
fn extract_refno_enum(value: &NamedAttrValue) -> Option<RefnoEnum> {
    match value {
        NamedAttrValue::RefU64Type(r) => Some((*r).clone().into()),
        NamedAttrValue::RefnoEnumType(r) => Some(*r),
        NamedAttrValue::RefU64Array(arr) => arr.first().cloned(),
        _ => None,
    }
}

#[cached(result = true)]
pub async fn get_cat_attmap(refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    let sql = format!(
        r#"
        (select value [{CATR_QUERY_STR}][where noun in ["SCOM", "SPRF", "SFIT", "JOIN", "SPCO"]].refno.*
        from only {} limit 1 fetch SCOM)[0] "#,
        refno.to_pe_key()
    );
    let result: Option<NamedAttrMap> = SUL_DB.query_take(&sql, 0).await?;
    Ok(result.unwrap_or_default())
}

/// 获取直接子节点的属性映射
///
/// # 注意
/// **已重构**: 现在使用 `collect_children_filter_attrs` 实现
#[cached(result = true)]
pub async fn get_children_named_attmaps(refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
    use crate::graph::collect_children_filter_attrs;
    collect_children_filter_attrs(refno, &[]).await
}

///获取所有直接子节点的完整元素
///
/// # 注意
/// **已重构**: 现在使用 `collect_children_elements` 实现
#[cached(result = true)]
pub async fn get_children_pes(refno: RefnoEnum) -> anyhow::Result<Vec<SPdmsElement>> {
    use crate::graph::collect_children_elements;
    collect_children_elements(refno, &[]).await
}

///传入一个负数的参考号数组，返回一个数组，包含所有子孙的EleTreeNode
// #[cached(result = true)]
pub async fn get_children_ele_nodes(refno: RefnoEnum) -> anyhow::Result<Vec<EleTreeNode>> {
    let sql = format!(
        r#"
        select refno, noun, name, owner, 0 as order,
                        op?:0 as op,
                        array::len((select value refnos from only type::record("his_pe", record::id($self.id)))?:[]) as mod_cnt,
                        array::len(children) as children_count,
                        status_code as status_code
                    from {}.children where id!=none and record::exists(id) and !deleted
        "#,
        refno.to_pe_key()
    );
    //
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let mut nodes: Vec<EleTreeNode> = response.take(0)?;
    //检查名称，如果没有给名字的，需要给上默认值, todo 后续如果是删除了又增加，名称后面的数字可能会继续增加
    let mut hashmap: HashMap<&str, i32> = HashMap::new();
    for node in &mut nodes {
        if node.name.is_empty() {
            let mut n = 1;
            if let Some(k) = hashmap.get_mut(&node.noun.as_str()) {
                *k += 1;
                n = *k;
            } else {
                hashmap.insert(node.noun.as_str(), 1);
            }
            node.name = format!("{} {}", node.noun.as_str(), n);
        }
    }
    Ok(nodes)
}

pub async fn clear_all_caches(refno: RefnoEnum) {
    // crate::GET_WORLD_TRANSFORM.lock().await.cache_remove(&refno);
    // 暂时注释掉，等待 cached 宏类型推断问题解决
    // crate::GET_WORLD_TRANSFORM.lock().await.cache_clear();
    // crate::GET_WORLD_MAT4.lock().await.cache_clear();
    QUERY_ANCESTOR_REFNOS.lock().await.cache_remove(&refno);
    QUERY_DEEP_CHILDREN_REFNOS.lock().await.cache_remove(&refno);
    GET_PE.lock().await.cache_remove(&refno);
    GET_TYPE_NAME.lock().await.cache_remove(&refno);
    GET_SIBLINGS.lock().await.cache_remove(&refno);
    GET_NAMED_ATTMAP.lock().await.cache_remove(&refno);
    // GET_ANCESTOR_ATTMAPS.lock().await.cache_remove(&refno);
    GET_NAMED_ATTMAP_WITH_UDA.lock().await.cache_remove(&refno);
    GET_CHILDREN_REFNOS.lock().await.cache_remove(&refno);
    GET_CHILDREN_NAMED_ATTMAPS.lock().await.cache_remove(&refno);
    GET_CAT_ATTMAP.lock().await.cache_remove(&refno);
    GET_CAT_REFNO.lock().await.cache_remove(&refno);
    // GET_UI_NAMED_ATTMAP.lock().await.cache_remove(&refno);
    GET_CHILDREN_PES.lock().await.cache_remove(&refno);
}

///获得children
#[cached(result = true)]
pub async fn get_children_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    // 临时方案：跳过历史版本查询以避免 fn::ses_date() 导致的 "Expected any, got record" 错误
    // TODO: 使用 dt 字段替代 fn::ses_date() 来支持历史版本查询
    if !refno.is_latest() {
        eprintln!("警告: 跳过历史版本 {:?} 的子节点查询（临时方案）", refno);
        return Ok(vec![]);
    }

    let sql = format!(
        r#"select value in from {}<-pe_owner where in.id!=none and record::exists(in.id) and !in.deleted"#,
        refno.to_pe_key()
    );
    SUL_DB.query_take::<Vec<RefnoEnum>>(&sql, 0).await
}

pub async fn query_multi_children_refnos(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut final_refnos = vec![];
    for &refno in refnos {
        match get_children_refnos(refno).await {
            Ok(children) => {
                final_refnos.extend(children);
            }
            Err(e) => {
                eprintln!("获取子参考号时出错: refno={:?}, 错误: {:?}", refno, e);
                // 这里可以选择继续循环或返回错误
                return Err(e); // 如果要中断并返回错误
                // 或者跳过此错误项，继续处理下一个
            }
        };
    }
    Ok(final_refnos)
}

///按cata_hash 分组获得不同的参考号类型
// #[cached(result = true)]
pub async fn query_group_by_cata_hash(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
) -> anyhow::Result<DashMap<String, CataHashRefnoKV>> {
    let keys = refnos
        .into_iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>();
    let mut result_map: DashMap<String, CataHashRefnoKV> = DashMap::new();
    for chunk in keys.chunks(20) {
        let sql = format!(
            r#"
            let $a = array::flatten(select value array::flatten([id, <-pe_owner.in]) from [{}])[? noun!=NONE && !deleted];
            select [cata_hash, type::record('inst_info', cata_hash).id!=none,
                    type::record('inst_info', cata_hash).ptset] as k,
                 array::group(id) as v
            from $a where noun not in ["BRAN", "HANG"]  group by k;
        "#,
            chunk.join(",")
        );
        // println!("query_group_by_cata_hash sql is {}", &sql);
        let mut response: Response = SUL_DB.query_response(sql).await?;
        // dbg!(&response);
        // 使用专门的结构体接收查询结果
        let d: Vec<CataHashGroupQueryResult> = take_vec(&mut response, 1).unwrap();
        let map = d
            .into_iter()
            .map(
                |CataHashGroupQueryResult {
                     k: (cata_hash, exist_inst, ptset),
                     v: group_refnos,
                 }| {
                    (
                        cata_hash.clone(),
                        CataHashRefnoKV {
                            cata_hash,
                            group_refnos,
                            exist_inst,
                            ptset: ptset.map(|x| {
                                // ptset 现在是数组，需要转换为 BTreeMap<i32, CateAxisParam>
                                x.into_iter().map(|param| (param.number, param)).collect()
                            }),
                        },
                    )
                },
            )
            .collect::<DashMap<String, CataHashRefnoKV>>();
        for (k, v) in map {
            if result_map.contains_key(&k) {
                result_map
                    .get_mut(&k)
                    .unwrap()
                    .group_refnos
                    .extend(v.group_refnos);
            } else {
                result_map.insert(k, v);
            }
        }
    }
    Ok(result_map)
}

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize, SurrealValue)]
pub struct PdmsSpreName {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub foreign_refno: Option<String>,
    pub name: Option<String>,
}

/// 查询多个参考号外键对应的name，暂时只支持SPRE这种一层外键的
pub async fn query_foreign_refnos(
    refnos: Vec<RefU64>,
    foreign_type: &str,
) -> anyhow::Result<Vec<PdmsSpreName>> {
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "select refno, refno.{} as foreign_refno,refno.{}.refno.NAME as name from [{}];",
        &foreign_type, &foreign_type, refnos
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let result: Vec<PdmsSpreName> = response.take(0)?;
    Ok(result)
}

pub async fn query_single_by_paths(
    refno: RefnoEnum,
    paths: &[&str],
    fields: &[&str],
) -> anyhow::Result<NamedAttrMap> {
    let mut ps = vec![];
    for &path in paths {
        let p = path.replace("->", ".refno.");
        let str = if p.starts_with(".") {
            p[1..].to_owned()
        } else {
            p
        };
        ps.push(str);
    }
    let sql = format!(
        r#"(select value refno.* from (select value [{}] from only {}) where id != none)[0]"#,
        ps.join(","),
        refno.to_pe_key()
    );
    #[cfg(feature = "debug_model")]
    println!("query_single_by_paths Sql is {}", sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let mut map = response
        .take::<Option<NamedAttrMap>>(0)?
        .unwrap_or_default();
    // dbg!(&map);
    //只保留 fileds 里的数据
    if !fields.is_empty() {
        map.retain(|k, _| fields.contains(&k.as_str()));
    }
    // dbg!(&map);
    Ok(map)
}

///通过类型过滤所有的参考号
pub async fn query_refnos_by_type(noun: &str, module: DBType) -> anyhow::Result<Vec<RefU64>> {
    let mdb = crate::get_db_option().mdb_name.clone();
    let dbnums = query_mdb_db_nums(Some(mdb), module).await?;
    let sql = format!(
        r#"select value id from {} where dbnum in [{}]"#,
        noun.to_uppercase(),
        dbnums.iter().map(|x| x.to_string()).join(",")
    );
    let mut refnos = SUL_DB.query_take(&sql, 0).await?;
    Ok(refnos)
}

/// 插入数据
pub async fn insert_into_table(db: &Surreal<Any>, table: &str, value: &str) -> anyhow::Result<()> {
    db.query(format!("insert ignore into {} {}", table, value))
        .await?;
    Ok(())
}

pub async fn insert_pe_into_table_with_chunks(
    db: &Surreal<Any>,
    table: &str,
    value: Vec<PbsElement>,
) -> anyhow::Result<()> {
    for r in value.chunks(MAX_INSERT_LENGTH) {
        let json = r.iter().map(|x| x.gen_sur_json()).join(",");
        let mut r = db
            .query(format!("insert ignore into {} [{}]", table, json))
            .await?;
        let mut error = r.take_errors();
        if !error.is_empty() {
            dbg!(&error);
        }
    }
    Ok(())
}

pub async fn insert_into_table_with_chunks<T>(
    db: &Surreal<Any>,
    table: &str,
    value: Vec<T>,
) -> anyhow::Result<()>
where
    T: Sized + Serialize,
{
    for r in value.chunks(MAX_INSERT_LENGTH) {
        let json = serde_json::to_string(r)?;
        db.query(format!("insert ignore into {} {}", table, json))
            .await?;
    }
    Ok(())
}

/// 批量插入relate数据，需要事先定义好每一条relate语句，并放到集合中
pub async fn insert_relate_to_table(db: &Surreal<Any>, value: Vec<String>) -> anyhow::Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let mut sql = String::new();
    for v in value {
        sql.push_str(&format!("{} ;", v));
    }
    sql.remove(sql.len() - 1);
    let mut r = db.query(&sql).await?;
    let mut error = r.take_errors();
    // if sql.contains("pbs:24381_101383"){
    //     dbg!(&sql);
    // }
    if !error.is_empty() {
        dbg!(&error);
    }
    Ok(())
}

/// 通过name查询参考号
pub async fn query_refnos_from_names(
    db: &Surreal<Any>,
    names: &Vec<String>,
) -> anyhow::Result<HashMap<String, PdmsElement>> {
    // 如果name不带 '/' 就加上 '/'
    let names = names
        .into_iter()
        .map(|name| {
            if name.starts_with("/") {
                name.to_string()
            } else {
                format!("/{}", name)
            }
        })
        .collect::<Vec<_>>();
    let names = serde_json::to_string(&names)?;
    let sql = format!(
        "select refno,name,noun,owner,0 as children_count , 0 as version, 0 as order from pe where name in {} and !deleted",
        names
    );
    let mut r = db.query(sql).await?;
    let eles: Vec<EleTreeNode> = r.take(0)?;
    let mut map = HashMap::new();
    for ele in eles {
        map.entry(ele.name.clone()).or_insert(ele.into());
    }
    Ok(map)
}

///查找所有同类型的参考号, 需要限制范围
pub async fn query_same_type_refnos(
    refno: RefnoEnum,
    mdb: String,
    module: DBType,
    get_owner: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let dbnums = query_mdb_db_nums(Some(mdb.clone()), module).await?;
    let mut sql = format!(
        r#"select value id from type::table({}.noun) where REFNO.dbnum in [{}] and !deleted"#,
        refno.to_pe_key(),
        dbnums.iter().map(|x| x.to_string()).join(",")
    );
    if get_owner {
        sql = sql.replace("value id", "value owner");
    }
    // println!("query_same_refnos_by_type sql: {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    Ok(refnos)
}

pub async fn query_types(refnos: &[RefU64]) -> anyhow::Result<Vec<Option<String>>> {
    let sql = format!(
        r#"select value noun from [{}]"#,
        refnos.iter().map(|x| x.to_pe_key()).join(",")
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let type_names: Vec<Option<String>> = response.take(0)?;
    Ok(type_names)
}

/// 查询管件的长度
pub async fn query_bran_fixing_length(refno: RefU64) -> anyhow::Result<f32> {
    let sql = format!(
        "return math::fixed(fn::bran_comp_len({})?:0.0,2)",
        refno.to_pe_key()
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let length: Option<f32> = response.take(0)?;
    Ok(length.unwrap_or(0.0))
}

//select value id from only pe_ses_h:['17496_171606', 0]..['17496_171606'];

/// 查询历史pe
pub async fn query_history_pes(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let refno_str = refno.refno().to_string();
    let mut response = SUL_DB
        .query(format!(
            r#"
            select value id from only pe_ses_h:['{0}', 0]..['{0}'];
        "#,
            refno_str,
        ))
        .await?;
    let pes: Vec<RefnoEnum> = response.take(0)?;
    Ok(pes)
}

/// 通过数据库查询refno离参考 sesno 最近的 sesno 数据
pub async fn query_refno_sesno(
    refno: RefU64,
    sesno: u32,
    dbnum: i32,
) -> anyhow::Result<(u32, u32)> {
    let sql = format!(
        "fn::latest_pe_sesno({}, {}, {})",
        refno.to_pe_key(),
        sesno,
        dbnum
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let r: Vec<u32> = response.take(0).unwrap();
    Ok((r[0], r[1]))
}

///查询历史数据的日期
pub async fn query_his_dates(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
) -> anyhow::Result<BTreeMap<RefnoEnum, NaiveDateTime>> {
    let refnos: Vec<_> = refnos.into_iter().collect();
    let pes = to_table_keys!(refnos.iter(), "pe");
    let his_refnos = to_table_keys!(refnos.iter(), "his_pe");
    let sql = format!(
        "select id as k, fn::ses_date(id) as v from array::flatten([{0}].refnos), [{1}];",
        his_refnos.join(","),
        pes.join(","),
    );
    // println!("query_his_dates sql: {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let r: Vec<KV<RefnoEnum, surrealdb::types::Datetime>> = response.take(0)?;
    Ok(r.into_iter().map(|kv| (kv.k, kv.v.naive_local())).collect())
}

/// 查询最新的参考号, 需要限制日期
pub async fn query_latest_refnos(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    dt: NaiveDateTime,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let pes = to_table_keys!(refnos, "pe");
    let sql = format!(
        "select value fn::find_pe_by_datetime(id, d'{}') from [{}]",
        dt.and_utc().to_rfc3339(),
        pes.join(","),
    );
    // println!("query_latest_refnos sql: {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let r: Vec<RefnoEnum> = response.take(0)?;
    Ok(r)
}

/// 获取参考号对应uda的数据
pub async fn get_uda_value(refno: RefU64, uda: &str) -> anyhow::Result<Option<String>> {
    let uda = uda.replace(":", "/");
    let sql = format!(
        "select value fn::get_uda_value(id,'{}') from {}",
        uda,
        refno.to_pe_key()
    );
    let mut resp: Response = SUL_DB.query_response(sql).await?;
    let r = resp.take::<Vec<Option<String>>>(0)?;
    if r.is_empty() {
        return Ok(None);
    }
    Ok(r[0].clone())
}

/// 按 NOUN 类型查询层次结构数据
///
/// 直接查询 NOUN 对应的类型表（如 SITE、ZONE、PIPE、BRAN、NOZZ 表），
/// 返回每个记录的 name、id、noun、owner_name、owner 和最后修改日期。
///
/// # 参数
/// * `noun` - 要查询的 NOUN 类型（如 "SITE", "ZONE", "PIPE", "BRAN", "NOZZ"）
/// * `name_filter` - 可选的名称过滤关键字，使用 `string::contains` 进行模糊匹配（匹配 NAME 字段）
/// * `parent_refnos` - 可选的父节点参考号列表，当提供时只查询这些父节点的直接子元素
///
/// # 返回值
/// * `Result<Vec<NounHierarchyItem>>` - 成功时返回匹配的记录列表
///
/// # 错误
/// 如果查询失败，返回错误信息
///
/// # 示例
/// ```no_run
/// use aios_core::{pe_key, query_noun_hierarchy};
///
/// # async fn example() -> anyhow::Result<()> {
/// // 查询所有 SITE
/// let sites = query_noun_hierarchy("SITE", None, None).await?;
///
/// // 查询名称包含 "107" 的 NOZZ
/// let nozzles = query_noun_hierarchy("NOZZ", Some("107"), None).await?;
///
/// // 查询指定父节点下的 PIPE
/// let some_parent = pe_key!("12345_6789");
/// let pipes = query_noun_hierarchy("PIPE", None, Some(vec![some_parent])).await?;
/// # Ok(())
/// # }
/// ```
pub async fn query_noun_hierarchy(
    noun: &str,
    name_filter: Option<&str>,
    parent_refnos: Option<Vec<RefnoEnum>>,
) -> anyhow::Result<Vec<NounHierarchyItem>> {
    let sanitized_filter = name_filter.map(|filter| filter.replace('\'', "\\'"));

    if let Some(parent_refnos) = parent_refnos {
        let mut aggregated_items = Vec::new();

        for parent_refno in parent_refnos {
            let name_filter_clause = sanitized_filter
                .as_ref()
                .map(|filter| {
                    format!(
                        "        AND string::contains(name ?? '', '{}')",
                        filter
                    )
                })
                .unwrap_or_default();

            let sql = format!(
                r#"
        SELECT
            fn::default_name(id) as name,
            id,
            noun,
            array::len(children) as children_cnt,
            fn::default_name(owner) as owner_name,
            owner as owner,
            IF fn::ses_date(id) != NONE THEN <datetime> fn::ses_date(id) ELSE NONE END as last_modified_date
        FROM {parent}.children
        WHERE id != none
            AND record::exists(id)
            AND !deleted
            AND noun = '{noun}'
{name_filter_clause}
        "#,
                parent = parent_refno.to_pe_key(),
                noun = noun,
                name_filter_clause = name_filter_clause
            );

            // 打印 SQL 以便调试
            println!("执行 SQL:\n{}", sql);

            let mut items = SUL_DB
                .query_take::<Vec<NounHierarchyItem>>(&sql, 0)
                .await?;
            aggregated_items.append(&mut items);
        }

        Ok(aggregated_items)
    } else {
        let where_clause = if let Some(filter) = sanitized_filter {
            format!(
                "WHERE REFNO!=NONE AND NAME != none AND string::contains(NAME, '{}')",
                filter
            )
        } else {
            "WHERE REFNO!=NONE ".to_string()
        };

        let sql = format!(
            r#"
        SELECT
            fn::default_name(REFNO) as name,
            REFNO as id,
            TYPE as noun,
            array::len(REFNO.children) as children_cnt,
            fn::default_name(REFNO.owner) as owner_name,
            REFNO.owner as owner,
            IF fn::ses_date(REFNO) != NONE THEN <datetime> fn::ses_date(REFNO) ELSE NONE END as last_modified_date
        FROM {noun}
        {where_clause}
        "#,
            noun = noun,
            where_clause = where_clause
        );

        // 打印 SQL 以便调试
        println!("执行 SQL:\n{}", sql);

        SUL_DB.query_take::<Vec<NounHierarchyItem>>(&sql, 0).await
    }
}

//添加query_his_dates 的 testcase
mod test {
    use std::str::FromStr;

    use chrono::NaiveDateTime;

    use crate::{init_test_surreal, pe_key, query_his_dates};

    #[tokio::test]
    async fn test_query_his_dates() {
        init_test_surreal().await;

        let r = query_his_dates(&[pe_key!("17496_172825")]).await.unwrap();
        dbg!(&r);
    }

    #[tokio::test]
    async fn test_query_latest_refnos() {
        init_test_surreal().await;

        //2025-07-03T07:18:52Z
        let r = crate::query_latest_refnos(
            &[pe_key!("17496_172825")],
            NaiveDateTime::from_str("2025-07-03T07:18:52Z").unwrap(),
        )
        .await
        .unwrap();
        dbg!(&r);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], pe_key!("17496_172825"));

        let r = crate::query_latest_refnos(
            &[pe_key!("17496_172825")],
            NaiveDateTime::from_str("2022-07-03T07:18:52Z").unwrap(),
        )
        .await
        .unwrap();
        dbg!(&r);
        assert_eq!(r.len(), 0);
    }

    #[tokio::test]
    async fn test_query_noun_hierarchy_pipe_tg105() {
        init_test_surreal().await;

        // 查询 PIPE 类型中名称包含 "TG-105" 的记录
        let result = crate::query_noun_hierarchy("PIPE", Some("TG-105"), None).await;

        match result {
            Ok(items) => {
                println!("找到 {} 条匹配的记录:", items.len());
                for (i, item) in items.iter().enumerate() {
                    println!("\n记录 {}:", i + 1);
                    println!("  名称: {}", item.name);
                    println!("  类型: {}", item.noun);
                    println!("  所有者: {:?}", item.owner);
                    println!("  最后修改日期: {:?}", item.last_modified_date);
                }
                dbg!(&items);

                if let Some(first) = items.first() {
                    let scoped = crate::query_noun_hierarchy(
                        "PIPE",
                        None,
                        Some(vec![first.owner]),
                    )
                    .await
                    .unwrap();
                    assert!(
                        scoped.iter().all(|item| item.owner == first.owner),
                        "Scoped query返回的所有元素都应该属于同一父节点"
                    );
                    assert!(
                        scoped.iter().any(|item| item.id == first.id),
                        "Scoped query 应该包含原查询中的元素"
                    );
                }
            }
            Err(e) => {
                eprintln!("查询失败: {}", e);
                panic!("查询失败: {}", e);
            }
        }
    }
}
