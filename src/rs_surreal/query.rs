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
use crate::consts::MAX_INSERT_LENGTH;
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
    pub k: (String, bool, Option<BTreeMap<String, CateAxisParam>>),
    pub v: Vec<RefnoEnum>,
}

///通过surql查询pe数据
#[cached(result = true)]
pub async fn get_pe(refno: RefnoEnum) -> anyhow::Result<Option<SPdmsElement>> {
    let sql = format!(
        r#"select * omit id from only {} limit 1;"#,
        refno.to_pe_key()
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let pe: Option<SPdmsElement> = response.take(0)?;
    Ok(pe)
}

pub async fn get_default_name(refno: RefnoEnum) -> anyhow::Result<Option<String>> {
    let sql = format!("return fn::default_name({});", refno.to_pe_key());
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let pe: Option<String> = response.take(0)?;
    Ok(pe)
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
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let s = response.take::<Vec<RefnoEnum>>(0);
    Ok(s?)
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
#[cached(result = true)]
pub async fn query_ancestor_of_type(
    refno: RefnoEnum,
    ancestor_type: String,
) -> anyhow::Result<Option<RefnoEnum>> {
    let sql = format!(
        "return fn::find_ancestor_type({}, '{}');",
        refno.to_pe_key(),
        ancestor_type
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let ancestor: Option<RefnoEnum> = response.take(0)?;
    Ok(ancestor)
}

// #[cached(result = true)]
/// 通过名称查询refno
///
/// # 参数
/// * `name` - 要查询的名称
///
/// # 返回值
/// * `Option<RefnoEnum>` - 如果找到则返回对应的refno,否则返回None
///
/// # 错误
/// * 如果查询失败会返回错误
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
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let s = response.take::<Vec<String>>(0);
    Ok(s?)
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
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let raw_values: Vec<SurlValue> = response.take(0)?;
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
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let type_name: Option<String> = response.take(0)?;
    Ok(type_name.unwrap_or("unset".to_owned()))
}

/// 批量获取多个refno的类型名称
///
/// # 参数
/// * `refnos` - refno迭代器
///
/// # 返回值
/// * `Vec<String>` - 类型名称列表
pub async fn get_type_names(
    refnos: impl Iterator<Item = &RefnoEnum>,
) -> anyhow::Result<Vec<String>> {
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let mut response = SUL_DB
        .query(format!(r#"select value noun from [{}]"#, pe_keys))
        .await?;
    let type_names: Vec<String> = response.take(0)?;
    Ok(type_names)
}

#[cached(result = true)]
pub async fn get_owner_type_name(refno: RefU64) -> anyhow::Result<String> {
    let sql = format!(
        "return (select value owner.noun from only (type::record('pe', {})));",
        refno.to_pe_key()
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    // dbg!(&response);
    let type_name: Option<String> = response.take(0)?;
    Ok(type_name.unwrap_or_default())
}

#[cached(result = true)]
pub async fn get_self_and_owner_type_name(refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
    let sql = format!(
        "select value [noun, owner.noun] from only {} limit 1",
        refno.to_pe_key()
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let type_name: Vec<String> = response.take(0)?;
    Ok(type_name)
}

///在父节点下的index, noun 有值时按照 noun 过滤
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
    // println!("sql is {}", &sql);

    let mut response: Response = SUL_DB.query_response(sql).await?;
    // dbg!(&response);
    let type_name: Option<u32> = response.take(0)?;
    Ok(type_name)
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct RefnoDatetime {
    pub refno: RefnoEnum,
    pub dt: Datetime,
}

///获取上一个版本的参考号
pub async fn query_prev_dt_refno(refno_enum: RefnoEnum) -> anyhow::Result<Option<RefnoDatetime>> {
    let sql = format!(
        "select old_pe as refno, fn::ses_date(old_pe) as dt from only {} where old_pe!=none limit 1;",
        refno_enum.to_pe_key(),
    );
    // println!("query_prev_version_refno sql is {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let refno: Option<RefnoDatetime> = response.take(0)?;
    Ok(refno)
}

///获取当前版本的参考号, 带日期的参考号
pub async fn query_dt_refno(refno_enum: RefnoEnum) -> anyhow::Result<Option<RefnoDatetime>> {
    let sql = format!(
        "select id as refno, fn::ses_date(id) as dt from only {} limit 1;",
        refno_enum.to_pe_key(),
    );
    // println!("query_dt_refno sql is {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let refno: Option<RefnoDatetime> = response.take(0)?;
    Ok(refno)
}

// //获取上一个版本的属性数据
pub async fn get_ui_named_attmap_prev_version(
    refno_enum: RefnoEnum,
) -> anyhow::Result<NamedAttrMap> {
    if let Some(refno_datetime) = query_prev_dt_refno(refno_enum).await? {
        return get_ui_named_attmap(refno_datetime.refno).await;
    }
    Ok(NamedAttrMap::default())
}

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

pub async fn query_full_names_map(
    refnos: &[RefnoEnum],
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    let mut response = SUL_DB
        .query(format!(
            "select value fn::default_full_name(id) from [{}]",
            refnos.into_iter().map(|x| x.to_pe_key()).join(",")
        ))
        .await?;
    let names: Vec<String> = response.take(0)?;
    let map = IndexMap::from_iter(refnos.iter().cloned().zip(names));
    Ok(map)
}

pub async fn query_full_names(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<String>> {
    let mut response = SUL_DB
        .query(format!(
            "select value fn::default_full_name(id) from [{}]",
            refnos.into_iter().map(|x| x.to_pe_key()).join(",")
        ))
        .await?;
    let names: Vec<String> = response.take(0)?;
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
    let mut response = SUL_DB
        .query(format!(
            "select value fn::default_full_name(id) from [{}]",
            refnos.into_iter().map(|x| x.to_pe_key()).join(",")
        ))
        .await?;
    let names: Vec<String> = response.take(0)?;
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
    let mut response = SUL_DB
        .query(format!(
            "select value fn::default_full_name(id) from [{}]",
            refnos.into_iter().map(|x| x.to_pe_key()).join(",")
        ))
        .await?;
    let names: Vec<String> = response.take(0)?;
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
                        if *n == 623723 {
                            vec.push(db1_dehash(*v as u32));
                        } else {
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
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let named_attmap: Option<NamedAttrMap> = response.take(0)?;
    Ok(named_attmap.unwrap_or_default())
}

#[cached(result = true)]
pub async fn get_siblings(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!("select value in from {}<-pe_owner", refno.to_pe_key());
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    Ok(refnos)
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
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let result: Option<String> = response.take(0)?;

    Ok(result.unwrap_or_default())
}

///通过surql查询属性数据，包含UDA数据
#[cached(result = true)]
pub(crate) async fn get_named_attmap_with_uda(
    refno_enum: RefnoEnum,
) -> anyhow::Result<NamedAttrMap> {
    let sql = format!(
        r#"
        --通过传递refno，查询属性值
        select fn::default_full_name(REFNO) as NAME, * from only {0}.refno fetch pe;
        select string::concat(':', if UDNA==none || string::len(UDNA)==0 {{ DYUDNA }} else {{ UDNA }}) as u, DFLT as v, UTYP as t from UDA where !UHIDE and {0}.noun in ELEL;
        -- uda 单独做个查询？
        select string::concat(':', if u.UDNA==none || string::len( u.UDNA)==0 {{ u.DYUDNA }} else {{ u.UDNA }}) as u, u.UTYP as t, v from (ATT_UDA:{1}).udas where u.UTYP != none;
        "#,
        refno_enum.to_pe_key(),
        refno_enum.refno()
    );

    let mut response: Response = SUL_DB.query_response(sql).await?;

    #[derive(Deserialize, SurrealValue)]
    struct AttrKV {
        u: String,
        t: String,
        v: SurlValue,
    }
    //获得uda的 map
    // dbg!(&response);
    let mut named_attmap = response
        .take::<Option<NamedAttrMap>>(0)?
        .unwrap_or_default();
    // dbg!(&named_attmap);
    let uda_kvs: Vec<AttrKV> = response.take(1)?;
    for AttrKV {
        u: uname,
        t: utype,
        v,
    } in uda_kvs
    {
        if uname.as_str() == ":NONE" || uname.as_str() == ":unset" || uname.is_empty() {
            continue;
        }
        let att_value = NamedAttrValue::from((utype.as_str(), v));
        named_attmap.insert(uname, att_value);
    }
    let overwrite_kvs: Vec<AttrKV> = response.take(2)?;
    for AttrKV {
        u: uname,
        t: utype,
        v,
    } in overwrite_kvs
    {
        if uname.as_str() == ":NONE" || uname.as_str() == ":unset" || uname.is_empty() {
            continue;
        }
        let att_value = NamedAttrValue::from((utype.as_str(), v));
        named_attmap.insert(uname, att_value);
    }
    Ok(named_attmap)
}

pub const CATR_QUERY_STR: &'static str = "refno.CATR.refno.CATR, refno.CATR.refno.PRTREF.refno.CATR, refno.SPRE, refno.SPRE.refno.CATR, refno.CATR";

#[cached(result = true)]
pub async fn get_cat_refno(refno: RefnoEnum) -> anyhow::Result<Option<RefnoEnum>> {
    let sql = format!(
        r#"
        select value [{CATR_QUERY_STR}][where noun in ["SCOM", "SPRF", "SFIT", "JOIN"]]
        from only {} limit 1;
    "#,
        refno.to_pe_key()
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let r: Option<RefnoEnum> = response.take(0)?;
    Ok(r)
}

#[cached(result = true)]
pub async fn get_cat_attmap(refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    crate::debug_model_debug!("🔍 get_cat_attmap for refno: {}", refno);
    let sql = format!(
        r#"
        (select value [{CATR_QUERY_STR}][where noun in ["SCOM", "SPRF", "SFIT", "JOIN"]].refno.*
        from only {} limit 1 fetch SCOM)[0] "#,
        refno.to_pe_key()
    );
    crate::debug_model_debug!("   SQL: {}", sql);
    // dbg!(&sql);
    // println!("sql is {}", &sql);
    let mut response: Response = SUL_DB.query_response(sql).await?;
    // dbg!(&response);
    #[derive(Deserialize)]
    struct AttrKV {
        u: String,
        t: String,
        v: SurlValue,
    }

    let result: anyhow::Result<NamedAttrMap> = take_single(&mut response, 0);
    match &result {
        Ok(named_attmap) => {
            crate::debug_model_debug!(
                "   ✅ 成功获取 cat_attmap, refno: {}",
                named_attmap.get_refno_or_default()
            );
        }
        Err(e) => {
            crate::debug_model_debug!("   ❌ 获取 cat_attmap 失败: {}", e);
        }
    }
    result
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
    crate::GET_WORLD_TRANSFORM.lock().await.cache_clear();
    crate::GET_WORLD_MAT4.lock().await.cache_clear();
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
        r#"select value in from {}<-pe_owner  where in.id!=none and record::exists(in.id) and !in.deleted"#,
        refno.to_pe_key()
    );
    let mut response: Response = SUL_DB.query_response(sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    Ok(refnos)
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
                                x.into_iter()
                                    .filter_map(|(k, v)| {
                                        // 尝试直接解析为 i32
                                        if let Ok(key) = k.parse::<i32>() {
                                            Some((key, v))
                                        } else if let Ok(refno) = RefU64::from_str(&k) {
                                            // 如果是 RefU64 格式（如 pe:⟨21895_68780⟩），转换为 i32
                                            Some((refno.0 as i32, v))
                                        } else {
                                            eprintln!("Warning: Failed to parse ptset key: {}", k);
                                            None
                                        }
                                    })
                                    .collect()
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
}
