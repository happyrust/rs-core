use std::collections::{BTreeMap, HashMap};

use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::types::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use surrealdb::sql::Thing;

#[derive(Clone, Debug, Default, Deserialize)]
struct KV<K, V> {
    k: K,
    v: V,
}

///通过surql查询pe数据
pub async fn get_pe(refno: RefU64) -> anyhow::Result<Option<SPdmsElement>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_pe_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let pe: Option<SPdmsElement> = response.take(0)?;
    Ok(pe)
}

///查询到祖先节点列表
pub async fn get_ancestor(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_ancestor_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let s = response.take::<Vec<Thing>>(1)?;
    Ok(s.into_iter().map(|s| s.into()).collect())
}

///查询到祖先节点属性数据
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

pub async fn get_type_name(refno: RefU64) -> anyhow::Result<String> {
    let mut response = SUL_DB
        .query(r#"return (select value noun from only (type::thing("pe", $refno)));"#)
        .bind(("refno", refno.to_string()))
        .await?;
    // dbg!(&response);
    let type_name: Option<String> = response.take(0)?;
    Ok(type_name.unwrap_or_default())
}

///查询的数据把 refno->name，换成名称
pub async fn get_ui_named_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let mut attmap = get_named_attmap_with_uda(refno, true).await?;
    attmap.fill_explicit_default_values();
    let mut refno_fields = vec![];
    let mut keys = vec![];
    let mut unset_keys = vec![];
    for (k, v) in &attmap.map {
        if k != "REFNO"
            && let NamedAttrValue::RefU64Type(r) = v
        {
            if r.is_valid() {
                refno_fields.push(r.to_pe_key());
                keys.push(k.to_owned());
            } else {
                unset_keys.push(k.to_owned());
            }
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
        attmap.insert(k, NamedAttrValue::StringType(v));
    }
    for k in unset_keys {
        attmap.insert(k, NamedAttrValue::StringType("unset".to_owned()));
    }
    Ok(attmap)
}

///通过surql查询属性数据
pub async fn get_named_attmap(refno: RefU64) -> anyhow::Result<NamedAttrMap> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_attmap_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(0)?;
    let named_attmap: NamedAttrMap = o.into();
    Ok(named_attmap)
}

///通过surql查询属性数据，包含UDA数据
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
    // dbg!(&uda_kvs);
    for map in uda_kvs {
        let k = map.get("u").unwrap().get_val_as_string();
        let splits = k.split("::").collect::<Vec<_>>();
        let uname = splits[0];
        if uname == ":NONE" || uname.is_empty() {
            continue;
        }
        let utype = splits[1];
        // dbg!((uname, utype));
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
    // dbg!(&overite_kvs);
    for map in overite_kvs {
        let k = map.get("u").unwrap().get_val_as_string();
        let v = map.get("v").unwrap().clone();
        named_attmap.insert(k, v);
    }
    Ok(named_attmap)
}

pub async fn get_cat_refno(refno: RefU64) -> anyhow::Result<Option<RefU64>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_cata_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let r: Option<RefU64> = response.take(1)?;
    Ok(r)
}

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

pub async fn get_children_pes(refno: RefU64) -> anyhow::Result<Vec<SPdmsElement>> {
    let mut response = SUL_DB
        .query(include_str!("schemas/query_children_pes_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let pes: Vec<SPdmsElement> = response.take(0)?;
    Ok(pes)
}

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
            }else{
                hashmap.insert(node.noun.as_str(), 1);
            }
            node.name = format!("{} {}", node.noun.as_str(), n);
        }
    }
    Ok(nodes)
}

///获得children
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

///按cata_hash 分组获得不同的参考号类型
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

//后面可以写一个map的语法
//沿着path，找到目标refno，如果没有就是None
// pub async fn query_by_path<T: DeserializeOwned>(
//     refno: RefU64,
//     path: &str,
// ) -> anyhow::Result<Option<T>> {
//     let mut p = path.replace("->", ".refno.");
//     let str = if p.starts_with(".") {
//         &p[1..]
//     } else {
//         p.as_str()
//     };
//     let sql = format!(
//         r#"select value {} from only type::thing("pe", $refno)"#,
//         str
//     );
//     let mut response = SUL_DB.query(sql).bind(("refno", refno.to_string())).await?;
//     let r: Option<T> = response.take(0)?;
//     Ok(r)
// }

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
