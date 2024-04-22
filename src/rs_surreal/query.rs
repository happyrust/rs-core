use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::E;
use std::sync::Mutex;
use serde_json::Value;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use truck_polymesh::Attributes;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[derive(Clone, Debug, Default, Deserialize)]
struct KV<K, V> {
    k: K,
    v: V,
}

///通过surql查询pe数据
#[cached(result = true)]
pub async fn get_pe(refno: RefU64) -> anyhow::Result<Option<SPdmsElement>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_pe_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let pe: Option<SPdmsElement> = response.take(0)?;
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
        .query(include_str!("schemas/query_ancestor_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let s = response.take::<Vec<RefU64>>(1);
    Ok(s?)
}

///查询到祖先节点属性数据
#[cached(result = true)]
pub async fn get_ancestor_attmaps(refno: RefU64) -> anyhow::Result<Vec<NamedAttrMap>> {
    let mut response = SUL_DB
        .query(include_str!(
            "schemas/query_ancestor_attmaps_by_refno.surql"
        ))
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(1)?;
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
    Ok(type_name.unwrap_or_default())
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

///查询的数据把 refno->name，换成名称
#[cached(result = true)]
pub async fn get_ui_named_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let mut attmap = get_named_attmap_with_uda(refno, true).await?;
    attmap.fill_explicit_default_values();
    let mut refno_fields = vec![];
    let mut keys = vec![];
    let mut unset_keys = vec![];
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
            NamedAttrValue::InvalidType => {
                unset_keys.push(k.to_owned());
            }
            _ => {}
        }
    }
    // dbg!(&keys);
    // dbg!(&refno_fields);
    let mut response = SUL_DB
        .query(format!(
            "select value name from [{}]",
            refno_fields.join(",")
        ))
        .await?;
    let names: Vec<String> = response.take(0)?;
    // dbg!(&names);
    for (k, v) in keys.into_iter().zip(names) {
        attmap.insert(k, NamedAttrValue::StringType(if v.is_empty() {
            "unset".to_owned()
        } else {
            v
        }));
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
        .query(include_str!("schemas/query_attmap_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(0)?;
    let named_attmap: NamedAttrMap = o.into();
    Ok(named_attmap)
}

#[cached(result = true)]
pub async fn get_siblings(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query("select value in from (select * from type::thing('pe', $refno).owner<-pe_owner order by order_num) where in.deleted=false")
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
        Ok(siblings.get(pos + 1).unwrap_or_default())
    } else {
        if pos == 0 { return Ok(Default::default()); }
        Ok(siblings.get(pos - 1).unwrap_or_default())
    }
}

///通过surql查询属性数据，包含UDA数据
#[cached(result = true)]
pub async fn get_named_attmap_with_uda(
    refno: RefU64,
    default_unset: bool,
) -> anyhow::Result<NamedAttrMap> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_full_attmap_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    //获得uda的 map
    let o: SurlValue = response.take(0)?;
    let mut named_attmap: NamedAttrMap = o.into();
    let uda_kvs: Vec<NamedAttrMap> = response.take(2)?;
    for map in uda_kvs {
        let k = map.get("u").unwrap().get_val_as_string();
        let splits = k.split("::").collect::<Vec<_>>();
        let uname = splits[0];
        if uname == ":NONE" || uname.is_empty() {
            continue;
        }
        let utype = splits[1];
        let v = map.get("v").unwrap().clone();
        if matches!(&v, NamedAttrValue::InvalidType) {
            if default_unset {
                named_attmap.insert(uname.to_owned(), NamedAttrValue::InvalidType);
            } else {
                named_attmap.insert(uname.to_owned(), NamedAttrValue::get_default_val(utype));
            }
        } else {
            named_attmap.insert(uname.to_owned(), v);
        }
    }
    let overite_kvs: Vec<NamedAttrMap> = response.take(3)?;
    for map in overite_kvs {
        let k = map.get("u").unwrap().get_val_as_string();
        let v = map.get("v").unwrap().clone();
        named_attmap.insert(k, v);
    }
    Ok(named_attmap)
}

#[cached(result = true)]
pub async fn get_cat_refno(refno: RefU64) -> anyhow::Result<Option<RefU64>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_cata_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let r: Option<RefU64> = response.take(1)?;
    Ok(r)
}


#[cached(result = true)]
pub async fn get_cat_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_cata_attmap.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(1)?;
    // dbg!(&o);
    let named_attmap: NamedAttrMap = o.into();
    Ok(named_attmap)
}

#[cached(result = true)]
pub async fn get_children_named_attmaps(refno: RefU64) -> anyhow::Result<Vec<NamedAttrMap>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_children_attmap_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(0)?;
    // dbg!(&o);
    let os: Vec<SurlValue> = o.try_into().unwrap();
    let named_attmaps: Vec<NamedAttrMap> = os.into_iter().map(|x| x.into()).collect();
    Ok(named_attmaps)
}

#[cached(result = true)]
pub async fn get_children_pes(refno: RefU64) -> anyhow::Result<Vec<SPdmsElement>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_children_pes_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let pes: Vec<SPdmsElement> = response.take(0)?;
    Ok(pes)
}

#[cached(result = true)]
pub async fn get_children_ele_nodes(refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_children_nodes_by_refno.surql"))
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

///获得children
#[cached(result = true)]
pub async fn get_children_refnos(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_children_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let id: Option<String> = response.take(0)?;
    // dbg!(&id);
    if id.is_none() {
        return Err(anyhow::anyhow!("{refno} not exist"));
    }
    let refnos: Vec<RefU64> = response.take(1)?;
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
    refnos: &[RefU64],
) -> anyhow::Result<IndexMap<String, Vec<RefU64>>> {
    let keys = refnos.iter().map(|x| x.to_pe_thing()).collect::<Vec<_>>();
    let mut response = SUL_DB
        .query(include_str!("schemas/group_by_cata_hash.surql"))
        .bind(("refnos", keys))
        .await?;
    let d: Vec<KV<String, Vec<RefU64>>> = response.take(1)?;
    let map = d
        .into_iter()
        .map(|kv| {
            let k = kv.k.clone();
            let v: Vec<RefU64> = kv.v;
            (k, v)
        })
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
pub async fn query_foreign_refnos(refnos: Vec<RefU64>, foreign_type: &str) -> anyhow::Result<Vec<PdmsSpreName>> {
    let refnos = refnos.into_iter().map(|refno| refno.to_pe_key()).collect::<Vec<_>>();
    let sql = format!("select refno, refno.{} as foreign_refno,refno.{}.refno.NAME as name from {};", &foreign_type, &foreign_type, serde_json::to_string(&refnos).unwrap_or("[]".to_string()));
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

/// 插入数据
pub async fn insert_into_table(db: &Surreal<Any>, table: &str, value: &str) -> anyhow::Result<()> {
    db.query(format!("insert into {} {}", table, value)).await?;
    Ok(())
}

/// 批量插入relate数据，需要事先定义好每一条relate语句，并放到集合中
pub async fn insert_relate_to_table(db: &Surreal<Any>, value: Vec<String>) -> anyhow::Result<()> {
    if value.is_empty() { return Ok(()); }
    let mut sql = String::new();
    for v in value {
        sql.push_str(&format!("{} ;", v));
    }
    sql.remove(sql.len() - 1);
    db.query(sql).await?;
    Ok(())
}