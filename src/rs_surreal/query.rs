use crate::consts::MAX_INSERT_LENGTH;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::ssc_setting::PbsElement;
use crate::table::ToTable;
use crate::tool::db_tool::db1_dehash;
use crate::tool::math_tool::*;
use crate::{graph::QUERY_DEEP_CHILDREN_REFNOS, types::*};
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use cached::Cached;
use dashmap::DashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::{BTreeMap, HashMap};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

#[derive(Clone, Debug, Default, Deserialize)]
struct KV<K, V> {
    k: K,
    v: V,
}

///通过surql查询pe数据
#[cached(result = true)]
pub async fn get_pe(refno: RefU64) -> anyhow::Result<Option<SPdmsElement>> {
    let mut response = SUL_DB
        .query("(select * omit id from (type::thing('pe', $refno)))[0];")
        .bind(("refno", refno.to_string()))
        .await?;
    let pe: Option<SPdmsElement> = response.take(0)?;
    Ok(pe)
}

pub async fn get_default_name(refno: RefU64) -> anyhow::Result<Option<String>> {
    let mut response = SUL_DB
        .query("fn::default_name(type::thing('pe', $refno));")
        .bind(("refno", refno.to_string()))
        .await?;
    let pe: Option<String> = response.take(0)?;
    Ok(pe)
}

#[cached(result = true)]
pub async fn get_design_dbnos(mdb_name: String) -> anyhow::Result<Vec<i32>> {
    let mdb = if mdb_name.starts_with("/") {
        mdb_name
    } else {
        format!("/{}", mdb_name)
    };
    let mut response = SUL_DB
        .query("select value (select value DBNO from CURD.refno.* where STYP=1) from only MDB where NAME=$mdb limit 1")
        .bind(("mdb", mdb))
        .await?;
    let dbnos: Vec<i32> = response.take(0)?;
    Ok(dbnos)
}

///查询到祖先节点列表
#[cached(result = true)]
pub async fn get_ancestor(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query("fn::ancestor(type::thing('pe', $refno));")
        .bind(("refno", refno.to_string()))
        .await?;
    let s = response.take::<Vec<RefU64>>(0);
    Ok(s?)
}

// #[cached(result = true)]
pub async fn get_refno_by_name(name: &str) -> anyhow::Result<Option<RefU64>> {
    let sql = format!(
        r#"select value id from only pe where name="/{}" limit 1;"#,
        name
    );
    println!("sql is {}", &sql);
    let mut response = SUL_DB
        .query(sql)
        .await?;
    let s = response.take::<Option<RefU64>>(0);
    Ok(s?)
}

#[cached(result = true)]
pub async fn get_ancestor_types(refno: RefU64) -> anyhow::Result<Vec<String>> {
    let mut response = SUL_DB
        .query("fn::ancestor(type::thing('pe', $refno)).noun;")
        .bind(("refno", refno.to_string()))
        .await?;
    let s = response.take::<Vec<String>>(0);
    Ok(s?)
}

///查询到祖先节点属性数据
#[cached(result = true)]
pub async fn get_ancestor_attmaps(refno: RefU64) -> anyhow::Result<Vec<NamedAttrMap>> {
    let mut response = SUL_DB
        .query("return fn::ancestor(type::thing('pe', $refno)).refno.*;")
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(0)?;
    let os: Vec<SurlValue> = o.try_into().unwrap();
    let named_attmaps: Vec<NamedAttrMap> = os.into_iter().map(|x| x.into()).collect();
    Ok(named_attmaps)
}

#[cached(result = true)]
pub async fn get_type_name(refno: RefU64) -> anyhow::Result<String> {
    let mut response = SUL_DB
        .query(r#"select value noun from only type::thing("pe", $refno)"#)
        .bind(("refno", refno.to_string()))
        .await?;
    let type_name: Option<String> = response.take(0)?;
    Ok(type_name.unwrap_or("unset".to_owned()))
}

pub async fn get_type_names(refnos: &[RefU64]) -> anyhow::Result<Vec<String>> {
    // let pe_keys = refnos.to_table_key("pe");
    let pe_keys = refnos.iter().map(|x| x.to_pe_key()).join(",");
    let mut response = SUL_DB
        .query(format!(r#"select value noun from [{}]"#, pe_keys))
        .await?;
    let type_names: Vec<String> = response.take(0)?;
    Ok(type_names)
}

#[cached(result = true)]
pub async fn get_owner_type_name(refno: RefU64) -> anyhow::Result<String> {
    let mut response = SUL_DB
        .query(r#"return (select value owner.noun from only (type::thing("pe", $refno)));"#)
        .bind(("refno", refno.to_string()))
        .await?;
    // dbg!(&response);
    let type_name: Option<String> = response.take(0)?;
    Ok(type_name.unwrap_or_default())
}

#[cached(result = true)]
pub async fn get_self_and_owner_type_name(refno: RefU64) -> anyhow::Result<Vec<String>> {
    let mut response = SUL_DB
        .query(r#"select value [noun, owner.noun] from only (type::thing("pe", $refno))"#)
        .bind(("refno", refno.to_string()))
        .await?;
    let type_name: Vec<String> = response.take(0)?;
    Ok(type_name)
}

///在父节点下的index, noun 有值时按照 noun 过滤
pub async fn get_index_by_noun_in_parent(
    parent: RefU64,
    refno: RefU64,
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

    let mut response = SUL_DB
        .query(sql)
        .await?;
    // dbg!(&response);
    let type_name: Option<u32> = response.take(0)?;
    Ok(type_name)
}

///查询的数据把 refno->name，换成名称
#[cached(result = true)]
pub async fn get_ui_named_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let mut attmap = get_named_attmap_with_uda(refno, true).await?;
    attmap.fill_explicit_default_values();
    let mut refno_fields = vec![];
    let mut keys = vec![];
    let mut unset_keys = vec![];
    let mut new_desp = None;
    let mut tuples = vec![];
    for (k, v) in &attmap.map {
        if k == "REFNO" {
            continue;
        }
        match v {
            NamedAttrValue::RefU64Type(r) => {
                if r.is_valid() {
                    refno_fields.push(r.to_pe_key());
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
                            &angles_to_dori(*d).unwrap_or_default(), false
                        )),
                    ));
                } else if k.contains("POS") {
                    tuples.push((k.clone(), NamedAttrValue::StringType(vec3_to_xyz_str(*d))));
                } else {
                    //默认是方向
                    tuples.push((
                        k.clone(),
                        NamedAttrValue::StringType(convert_to_xyz(&to_pdms_dvec_str(
                            &d.as_dvec3(), false
                        ))),
                    ));
                }
            }
            NamedAttrValue::F32VecType(d) => {
                if k == "DESP" {
                    let unip = attmap.get_i32_vec("UNIPAR").unwrap_or_default();
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

    let mut response = SUL_DB
        .query(format!(
            "select value fn::default_full_name(id) from [{}]",
            // "select value name from [{}]",
            refno_fields.join(",")
        ))
        .await?;
    let names: Vec<String> = response.take(0)?;
    for (k, v) in keys.into_iter().zip(names) {
        attmap.insert(
            k,
            NamedAttrValue::StringType(if v.is_empty() { "unset".to_owned() } else { v }),
        );
    }
    for k in unset_keys {
        attmap.insert(k, NamedAttrValue::StringType("unset".to_owned()));
    }
    Ok(attmap)
}

///通过surql查询属性数据
#[cached(result = true)]
pub async fn get_named_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let mut response = SUL_DB
        .query(r#"(select * from (type::thing("pe", $refno)).refno)[0];"#)
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(0)?;
    let named_attmap: NamedAttrMap = o.into();
    Ok(named_attmap)
}

#[cached(result = true)]
pub async fn get_siblings(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query("select value in from type::thing('pe', $refno).owner<-pe_owner")
        .bind(("refno", refno.to_string()))
        .await?;
    let refnos: Vec<RefU64> = response.take(0)?;
    Ok(refnos)
}

#[cached(result = true)]
pub async fn get_next_prev(refno: RefU64, next: bool) -> anyhow::Result<RefU64> {
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

///通过surql查询属性数据，包含UDA数据
#[cached(result = true)]
pub async fn get_named_attmap_with_uda(
    refno: RefU64,
    default_unset: bool,
) -> anyhow::Result<NamedAttrMap> {
    let mut response = SUL_DB
        .query(r#"
            --通过传递refno，查询属性值
    -- select fn::default_full_name(REFNO), * from only (type::thing("pe", $refno)).refno;
    select fn::default_full_name(REFNO) as NAME, * from only (type::thing("pe", $refno)).refno fetch pe;
    select string::concat(string::concat(string::concat(':', if UDNA==none || string::len(UDNA)==0 { DYUDNA } else { UDNA }), '::'), UTYP) as u, DFLT as v from UDA where !UHIDE and (type::thing("pe", $refno)).noun in ELEL;
    -- uda 单独做个查询？
    select if  u.UDNA==none || string::len( u.UDNA)==0 { u.DYUDNA } else { u.UDNA } as u, v from (type::thing("ATT_UDA", $refno)).udas;
        "#)
        .bind(("refno", refno.to_string()))
        .await?;
    //获得uda的 map
    let o: SurlValue = response.take(0)?;
    let mut named_attmap: NamedAttrMap = o.into();
    let uda_kvs: Vec<NamedAttrMap> = response.take(1)?;
    // dbg!(&uda_kvs);
    for map in uda_kvs {
        let k = map.get("u").unwrap().get_val_as_string();
        let splits = k.split("::").collect::<Vec<_>>();
        let uname = splits[0];
        if uname == ":NONE" || uname == ":unset" || uname.is_empty() {
            continue;
        }
        let utype = splits[1];
        // dbg!((uname, utype));
        let mut v = map.get("v").unwrap().clone();
        if matches!(&v, NamedAttrValue::InvalidType) {
            if default_unset {
                v = NamedAttrValue::InvalidType;
            } else {
                v = NamedAttrValue::get_default_val(utype);
            }
        }
        named_attmap.insert(uname.to_owned(), v);
    }
    let overwrite_kvs: Vec<NamedAttrMap> = response.take(2)?;
    // dbg!(&overite_kvs);
    for map in overwrite_kvs {
        let k = map.get("u").unwrap().get_val_as_string();
        if k == "NONE" || k == "unset" {
            continue;
        }
        let v = map.get("v").unwrap().clone();
        named_attmap.insert(format!(":{k}"), v);
    }
    Ok(named_attmap)
}

#[cached(result = true)]
pub async fn get_cat_refno(refno: RefU64) -> anyhow::Result<Option<RefU64>> {
    let mut response = SUL_DB
        .query(r#"
            select value [refno.CATR.refno.CATR, refno.CATR.refno.PRTREF.refno.CATR, refno.SPRE.refno.CATR, refno.CATR][where noun in ["SCOM", "SPRF", "SFIT", "JOIN"]]
            from only type::thing("pe", $refno) limit 1;
        "#)
        .bind(("refno", refno.to_string()))
        .await?;
    let r: Option<RefU64> = response.take(0)?;
    Ok(r)
}

#[cached(result = true)]
pub async fn get_cat_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let sql = format!(
        r#"
        (select value [refno.CATR.refno.CATR, refno.CATR.refno.PRTREF.refno.CATR, refno.SPRE.refno.CATR, refno.CATR][where noun in ["SCOM", "SPRF", "SFIT", "JOIN"]].refno.*
        from only {} limit 1 fetch SCOM)[0] "#, refno.to_pe_key());
    // dbg!(&sql);
    // println!("sql is {}", &sql);
    let mut response = SUL_DB
        .query(&sql)
        .await?;
    // dbg!(&response);
    let o: SurlValue = response.take(0)?;
    let named_attmap: NamedAttrMap = o.into();
    Ok(named_attmap)
}

#[cached(result = true)]
pub async fn get_children_named_attmaps(refno: RefU64) -> anyhow::Result<Vec<NamedAttrMap>> {
    let mut response = SUL_DB
        .query(
            "select value in.refno.* from (type::thing('pe', $refno))<-pe_owner where in.id!=none",
        )
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(0)?;
    // dbg!(&o);
    let os: Vec<SurlValue> = o.try_into().unwrap();
    let named_attmaps: Vec<NamedAttrMap> = os.into_iter().map(|x| x.into()).collect();
    Ok(named_attmaps)
}

///获取所有子孙的参考号
#[cached(result = true)]
pub async fn get_children_pes(refno: RefU64) -> anyhow::Result<Vec<SPdmsElement>> {
    let mut response = SUL_DB
        .query(r#"select value in.* from type::thing("pe", $refno)<-pe_owner where in.id!=none"#)
        .bind(("refno", refno.to_string()))
        .await?;
    let pes: Vec<SPdmsElement> = response.take(0)?;
    Ok(pes)
}

///传入一个负数的参考号数组，返回一个数组，包含所有子孙的参考号
pub async fn get_all_children_refnos(
    refnos: impl IntoIterator<Item = &RefU64>,
) -> anyhow::Result<Vec<RefU64>> {
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let sql = format!("array::flatten(select value in from [{pe_keys}]<-pe_owner)");
    let mut response = SUL_DB.query(sql).await?;
    let refnos: Vec<RefU64> = response.take(0)?;
    Ok(refnos)
}

///传入一个负数的参考号数组，返回一个数组，包含所有子孙的参考号
pub async fn query_filter_children(refno: RefU64, types: &[&str]) -> anyhow::Result<Vec<RefU64>> {
    let nouns_str = types
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = if types.is_empty() {
        format!(
            r#"select value in from {}<-pe_owner where in.id!=none"#,
            refno.to_pe_key()
        )
    } else {
        format!(
            r#"select value in from {}<-pe_owner where in.noun in [{nouns_str}] and in.id!=none"#,
            refno.to_pe_key()
        )
    };
    // println!("query_filter_children: {}", &sql);
    let mut response = SUL_DB.query(sql).await?;
    let pes: Vec<RefU64> = response.take(0)?;
    Ok(pes)
}

///传入一个负数的参考号数组，返回一个数组，包含所有子孙的EleTreeNode
#[cached(result = true)]
pub async fn get_children_ele_nodes(refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>> {
    let mut response = SUL_DB
        .query(r#"
            select in.refno as refno, in.noun as noun, in.name as name, in.owner as owner, array::first(in->pe_owner.id[1]) as order,
                 array::len(in<-pe_owner) as children_count from (type::thing("pe", $refno))<-pe_owner
        "#)
        .bind(("refno", refno.to_string()))
        .await?;
    let mut nodes: Vec<EleTreeNode> = response.take(0)?;
    //检查名称，如果没有给名字的，需要给上默认值, todo 后续如果是删除了又增加，名称后面的数字可能会继续增加
    let mut hashmap: HashMap<&str, i32> = HashMap::new();
    for node in &mut nodes {
        if node.name.is_empty() {
            // hashmap.entry(&node.noun).or_insert(1);
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

pub async fn clear_all_caches(refno: RefU64) {
    // crate::GET_WORLD_TRANSFORM.lock().await.cache_remove(&refno);
    crate::GET_WORLD_TRANSFORM.lock().await.cache_clear();
    crate::GET_WORLD_MAT4.lock().await.cache_clear();
    GET_ANCESTOR.lock().await.cache_remove(&refno);
    QUERY_DEEP_CHILDREN_REFNOS.lock().await.cache_remove(&refno);
    GET_PE.lock().await.cache_remove(&refno);
    GET_TYPE_NAME.lock().await.cache_remove(&refno);
    GET_SIBLINGS.lock().await.cache_remove(&refno);
    GET_NAMED_ATTMAP.lock().await.cache_remove(&refno);
    GET_ANCESTOR_ATTMAPS.lock().await.cache_remove(&refno);
    GET_NAMED_ATTMAP_WITH_UDA
        .lock()
        .await
        .cache_remove(&(refno, true));
    GET_NAMED_ATTMAP_WITH_UDA
        .lock()
        .await
        .cache_remove(&(refno, false));
    GET_CHILDREN_REFNOS.lock().await.cache_remove(&refno);
    GET_CHILDREN_NAMED_ATTMAPS.lock().await.cache_remove(&refno);
    GET_CAT_ATTMAP.lock().await.cache_remove(&refno);
    GET_CAT_REFNO.lock().await.cache_remove(&refno);
    GET_UI_NAMED_ATTMAP.lock().await.cache_remove(&refno);
    GET_CHILDREN_PES.lock().await.cache_remove(&refno);
}

///获得children
#[cached(result = true)]
pub async fn get_children_refnos(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query("select value in from type::thing('pe', $refno)<-pe_owner")
        .bind(("refno", refno.to_string()))
        .await?;
    let refnos: Vec<RefU64> = response.take(0)?;
    Ok(refnos)
}

pub async fn query_multi_children_refnos(refnos: &[RefU64]) -> anyhow::Result<Vec<RefU64>> {
    let mut refno_ids = refnos.iter().map(|x| x.to_pe_key()).collect::<Vec<_>>();
    let mut response = SUL_DB
        .query(format!(
            "array::flatten(select value in.id from [{}]<-pe_owner)",
            refno_ids.join(",")
        ))
        .await?;
    let refnos: Vec<RefU64> = response.take(0)?;
    Ok(refnos)
}

///按cata_hash 分组获得不同的参考号类型
// #[cached(result = true)]
pub async fn query_group_by_cata_hash(
    refnos: impl IntoIterator<Item = &RefU64>,
) -> anyhow::Result<DashMap<String, CataHashRefnoKV>> {
    let keys = refnos
        .into_iter()
        .map(|x| x.to_pe_thing())
        .collect::<Vec<_>>();
    let mut response = SUL_DB
        .query(r#"
            let $a = array::flatten(select value array::flatten([id, <-pe_owner.in]) from $refnos);
            select [cata_hash, type::thing('inst_info', cata_hash).id!=none,
                 type::thing('inst_info', cata_hash).ptset] as k, array::group(id) as v from $a group by k;
        "#)
        .bind(("refnos", keys))
        .await?;
    let d: Vec<KV<(String, bool, Option<BTreeMap<i32, CateAxisParam>>), Vec<RefU64>>> =
        response.take(1)?;
    let map = d
        .into_iter()
        .map(
            |KV {
                 k: (cata_hash, exist_inst, ptset),
                 v: group_refnos,
             }| {
                (
                    cata_hash.clone(),
                    CataHashRefnoKV {
                        cata_hash,
                        group_refnos,
                        exist_inst,
                        ptset,
                    },
                )
            },
        )
        .collect();
    Ok(map)
}

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
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
        .collect::<Vec<_>>();
    let sql = format!(
        "select refno, refno.{} as foreign_refno,refno.{}.refno.NAME as name from {};",
        &foreign_type,
        &foreign_type,
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<PdmsSpreName> = response.take(0)?;
    Ok(result)
}

pub async fn query_single_by_paths(
    refno: RefU64,
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
    let select_fieds = if fields.is_empty() {
        "*".to_string()
    } else {
        fields.join(",")
    };
    let sql = format!(
        r#"(select {} from (select value [{}] from only type::thing("pe", $refno)) where id != none)[0]"#,
        select_fieds,
        ps.join(",")
    );
    // #[cfg(debug_assertions)]
    // println!("Sql is {}", sql);
    let mut response = SUL_DB.query(sql).bind(("refno", refno.to_string())).await?;
    let r: Option<NamedAttrMap> = response.take(0)?;
    Ok(r.unwrap_or_default())
}

///通过类型过滤所有的参考号
pub async fn query_refnos_by_type(noun: &str) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query(format!(
            r#"select value meta::id(id) from {}"#,
            noun.to_uppercase()
        ))
        .await?;
    let refnos: Vec<RefU64> = response.take(0)?;
    Ok(refnos)
}

/// 插入数据
pub async fn insert_into_table(db: &Surreal<Any>, table: &str, value: &str) -> anyhow::Result<()> {
    db.query(format!("insert into {} {}", table, value)).await?;
    Ok(())
}

pub async fn insert_pe_into_table_with_chunks(
    db: &Surreal<Any>,
    table: &str,
    value: Vec<PbsElement>,
) -> anyhow::Result<()> {
    for r in value.chunks(MAX_INSERT_LENGTH) {
        let json = r.iter().map(|x| x.gen_sur_json()).join(",");
        db.query(format!("insert into {} [{}]", table, json))
            .await?;
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
        db.query(format!("insert into {} {}", table, json)).await?;
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
    db.query(sql).await?;
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
        "select refno,name,noun,owner,0 as children_count , 0 as version, 0 as order from pe where name in {}",
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
