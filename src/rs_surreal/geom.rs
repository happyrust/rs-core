use crate::pdms_types::*;
use crate::pe::SPdmsElement;
use crate::{query_filter_deep_children, types::*};
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

///通过surql查询pe数据
#[cached(result = true)]
pub async fn query_deep_inst_info_refnos(refno: RefU64) -> anyhow::Result<Vec<RefU64>> {
    let types = super::get_self_and_owner_type_name(refno).await?;
    // dbg!(&types);
    if types[1] == "BRAN" || types[1] == "HANG" {
        return Ok(vec![refno]);
    }
    if types[0] == "BRAN" || types[0] == "HANG" {
        let children_refnos = super::get_children_refnos(refno).await?;
        return Ok(children_refnos);
    }
    let branch_refnos = super::query_filter_deep_children(refno, &["BRAN", "HANG"]).await?;
    // dbg!(&branch_refnos);

    let mut target_refnos = super::query_multi_children_refnos(&branch_refnos).await?;

    let visible_refnos = super::query_filter_deep_children(refno, &VISBILE_GEO_NOUNS).await?;
    // dbg!(&visible_refnos);

    target_refnos.extend(visible_refnos);
    Ok(target_refnos)
}

//leave_or_arrive: true: leave, false: arrive
#[cached(result = true)]
pub async fn query_la_axis_attmap(
    refno: RefU64,
    leave_or_arrive: bool,
) -> anyhow::Result<NamedAttrMap> {
    // let cata_refno = super::get_cat_refno(refno).await?.ok_or(anyhow::anyhow!("no cat_refno"))?;
    // dbg!(&cata_refno);
    // let axis_map = super::query_single_by_paths(
    //     cata_refno,
    //     &["->PTRE", "->PTSE"],
    //     &["refno"],
    // )
    // .await?;
    Ok(Default::default())
}

/// 参考号具有正负实体映射关系的信息结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct RefnoHasNegPosInfo {
    // pub refno: RefU64,
    /// 正实体的参考号
    pub pos: RefU64,
    /// 负实体的参考号集合
    pub negs: Vec<RefU64>,
}

/// 后面再创建一个 compound 的关系，负责连接这个参考号对应的 info，并标记为 compound， compound 优先
/// 是
///返回有负实体和正实体的参考号集合，还有对应的NOUN
///还要考虑下面有多个LOOP或者PLOO的情况，第二个开始都是负实体
/// 首先查询到所有的负实体，然后找到sibling和父节点
pub async fn query_refno_has_pos_neg_map(
    refno: RefU64,
    is_cata: Option<bool>, //是否是元件库里的负实体查询
) -> anyhow::Result<HashMap<RefU64, Vec<RefU64>>> {
    //先查询负实体和它的neg children
    // let mut refno_ids = refnos.iter().map(|x| x.to_pe_key()).collect::<Vec<_>>();
    let nouns = match is_cata {
        Some(true) => CATE_NEG_NOUN_NAMES.as_slice(),
        Some(false) => &GENRAL_NEG_NOUN_NAMES.as_slice(),
        _ => &TOTAL_NEG_NOUN_NAMES.as_slice(),
    };
    //查询元件库下的负实体组合
    let refnos = query_filter_deep_children(refno, nouns).await.unwrap();
    // dbg!(&refnos);
    //使用SUL_DB通过这些参考号反过来query查找父节点
    let sql = format!(
         "select pos, array::group(id) as negs from (select $this.id as id, array::first(->pe_owner.out) as pos from [{}]) group pos",
         refnos.iter().map(|x| x.to_pe_key()).collect::<Vec<_>>().join(","),
     );
    //  dbg!(&sql);
    let mut response = SUL_DB.query(&sql).await?;
    // let r = response.take::<Vec<RefnoHasNegPosInfo>>(0).unwrap();
    //  dbg!(&response);
    let mut result = HashMap::new();
    if let Ok(r) = response.take::<Vec<RefnoHasNegPosInfo>>(0) {
        for info in r {
            result.insert(info.pos, info.negs);
        }
    }
    Ok(result)

}

/// 查询具有正负实体映射关系的参考号集合
///
/// # 参数
/// - `refno`: 参考号数组
/// - `is_cata`: 是否是元件库里的负实体查询
///
/// # 返回
/// 返回一个哈希映射，其中键是参考号，值是具有正负实体映射关系的参考号信息列表
pub async fn query_refnos_has_pos_neg_map(
    refno: &[RefU64],
    is_cata: Option<bool>,
) -> anyhow::Result<HashMap<RefU64, Vec<RefU64>>> {
    
    let mut result = HashMap::new();
    for &refno in refno {
        let mut map = query_refno_has_pos_neg_map(refno, is_cata).await?;
        result.extend(map.drain());
    }
    Ok(result)
}

