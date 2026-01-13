use crate::parsed_data::CateAxisParam;
use crate::pdms_pluggin::heat_dissipation::InstPointMap;
use crate::pe::SPdmsElement;
use crate::shape::pdms_shape::RsVec3;
use crate::utils::{take_option, take_vec};
use crate::vec3_pool::parse_ptset_auto;
use crate::{NamedAttrMap, RefnoEnum};
use crate::{SUL_DB, SurlValue, SurrealQueryExt};
use crate::{init_test_surreal, query_filter_deep_children, types::*};
use crate::{pdms_types::*, to_table_key, to_table_keys};
use bevy_transform::components::Transform;
use cached::proc_macro::cached;
use glam::Vec3;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use smol_str::ToSmolStr;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Mutex;
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;

/// 将 ptset JSON（原始或压缩）解码为按 number 组织的映射
#[inline]
fn decode_ptset_map(ptset_array: &Option<Value>) -> BTreeMap<String, CateAxisParam> {
    ptset_array
        .as_ref()
        .and_then(parse_ptset_auto)
        .unwrap_or_default()
        .into_iter()
        .map(|param| (param.number.to_string(), param))
        .collect()
}

//获得参考号对应的inst keys
pub fn get_inst_relate_keys(refnos: &[RefnoEnum]) -> String {
    if !refnos.is_empty() {
        refnos
            .iter()
            // 统一使用尖括号形式，避免生成 `inst_relate:17496_106028` 这类在 SurrealQL 中不稳定/不可解析的 key
            .map(|x| format!("inst_relate:⟨{}⟩", x))
            .collect::<Vec<_>>()
            .join(",")
        // format!("array::flatten([{pes}]->inst_relate)")
    } else {
        "inst_relate".to_string()
    }
}

/// 从 LOOP/PLOO 子元素获取顶点数据的响应结构（内部使用）
#[derive(Serialize, Deserialize, Debug, Default, SurrealValue)]
struct LoopHeightRaw {
    loops: Vec<RsVec3>,
    height: Option<f32>,
}

/// fetch_loops_and_height 函数的返回结构体
#[derive(Debug, Default, Clone)]
pub struct LoopHeightResult {
    /// 所有 loop 的顶点数据
    pub loops: Vec<Vec<Vec3>>,
    /// 高度值
    pub height: f32,
}

/// 获得当前参考号对应的loops（例如Panel下的loops，可能有多个）
///
/// 注意：顶点数据存储在 LOOP/PLOO 的子元素 PAVE/PONT 上，而不是 LOOP/PLOO 本身
pub async fn fetch_loops_and_height(refno: RefnoEnum) -> anyhow::Result<LoopHeightResult> {
    // 新查询：从 LOOP/PLOO 的子元素 PAVE/PONT 获取顶点数据
    let sql = format!(
        r#"SELECT value {{ 
            loops: (SELECT value [refno.POS[0], refno.POS[1], refno.FRAD] FROM id.children WHERE noun IN ["PAVE", "PONT"]), 
            height: refno.HEIG 
        }} FROM {0}.children WHERE noun IN ["LOOP", "PLOO"]"#,
        refno.to_pe_key()
    );
    // println!(" fetch_loops_and_height sql is {}", &sql);
    let mut response = SUL_DB.query_response(&sql).await.unwrap();
    let results: Vec<LoopHeightRaw> = response.take(0)?;

    // 提取所有 loop 的顶点和高度
    let mut all_loops: Vec<Vec<Vec3>> = Vec::new();
    let mut height: f32 = 0.0;

    for result in results {
        let points: Vec<Vec3> = result.loops.into_iter().map(|v| v.0).collect();
        if !points.is_empty() {
            all_loops.push(points);
        }
        // 使用第一个有效的高度值
        if height == 0.0 {
            height = result.height.unwrap_or_default();
        }
    }

    Ok(LoopHeightResult {
        loops: all_loops,
        height,
    })
}

///通过surql查询pe数据
#[cached(result = true)]
pub async fn query_deep_visible_inst_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let types = super::get_type_and_owner_type(refno).await?;
    if types[1] == "BRAN" || types[1] == "HANG" {
        return Ok(vec![refno]);
    }
    if types[0] == "BRAN" || types[0] == "HANG" {
        let children_refnos = super::get_children_refnos(refno).await?;
        return Ok(children_refnos);
    }
    //TODO，这里可以采用ZONE作为中间层去加速这个过程
    //按照所允许的层级关系去遍历？
    let branch_refnos = super::query_filter_deep_children(refno, &["BRAN", "HANG"]).await?;

    let mut target_refnos = super::query_multi_children_refnos(&branch_refnos).await?;

    let visible_refnos = super::query_filter_deep_children(refno, &VISBILE_GEO_NOUNS).await?;
    target_refnos.extend(visible_refnos);
    Ok(target_refnos)
}

#[cached(result = true)]
pub async fn query_deep_neg_inst_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let neg_refnos = super::query_filter_deep_children(refno, &TOTAL_NEG_NOUN_NAMES).await?;
    Ok(neg_refnos)
}

//leave_or_arrive: true: leave, false: arrive
#[cached(result = true)]
pub async fn query_la_axis_attmap(
    refno: RefnoEnum,
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
    // pub refno: RefnoEnum,
    /// 正实体的参考号
    pub pos: RefnoEnum,
    /// 负实体的参考号集合
    pub negs: Vec<RefnoEnum>,
}

/// 后面再创建一个 compound 的关系，负责连接这个参考号对应的 info，并标记为 compound， compound 优先
/// 是
///返回有负实体和正实体的参考号集合，还有对应的NOUN
///还要考虑下面有多个LOOP或者PLOO的情况，第二个开始都是负实体
/// 首先查询到所有的负实体，然后找到sibling和父节点
pub async fn query_refno_has_pos_neg_map(
    refno: RefnoEnum,
    is_cata: Option<bool>, //是否是元件库里的负实体查询
) -> anyhow::Result<HashMap<RefnoEnum, Vec<RefnoEnum>>> {
    //先查询负实体和它的neg children
    let nouns = match is_cata {
        Some(true) => CATE_NEG_NOUN_NAMES.as_slice(),
        Some(false) => &GENRAL_NEG_NOUN_NAMES.as_slice(),
        _ => &TOTAL_NEG_NOUN_NAMES.as_slice(),
    };
    //查询元件库下的负实体组合
    let refnos = query_filter_deep_children(refno, nouns).await.unwrap();
    if refnos.is_empty() {
        return Ok(HashMap::new());
    }
    //使用SUL_DB通过这些参考号反过来query查找父节点
    let sql = format!(
        "select pos, array::group(id) as negs from (select $this.id as id, array::first(->pe_owner.out) as pos from [{}]) group pos",
        refnos
            .iter()
            .map(|x| x.to_pe_key())
            .collect::<Vec<_>>()
            .join(","),
    );
    // println!("query_refno_has_pos_neg_map sql is {}", &sql);
    let mut response = SUL_DB.query_response(&sql).await?;
    let mut result = HashMap::new();
    if let Ok(infos) = take_vec::<RefnoHasNegPosInfo>(&mut response, 0) {
        for info in infos {
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
    refno: &[RefnoEnum],
    is_cata: Option<bool>,
) -> anyhow::Result<HashMap<RefnoEnum, Vec<RefnoEnum>>> {
    let mut result = HashMap::new();
    for &refno in refno {
        let mut map = query_refno_has_pos_neg_map(refno, is_cata).await?;
        result.extend(map.drain());
    }
    Ok(result)
}

/// 查询bran下所有元件的点集
pub async fn query_bran_children_point_map(refno: RefnoEnum) -> anyhow::Result<Vec<InstPointMap>> {
    // ptset 现在是数组，需要转换为 BTreeMap<String, CateAxisParam>
    let sql = format!(
        "select in.id as refno, out.ptset as ptset_array, in.noun as att_type from pe:{}<-pe_owner->inst_relate;",
        refno.to_string()
    );
    let mut response = SUL_DB.query_response(&sql).await?;
    let mut results: Vec<(RefnoEnum, Option<serde_json::Value>, String)> =
        take_vec(&mut response, 0)?;

    Ok(results
        .into_iter()
        .map(|(refno, ptset_array, att_type)| {
            let ptset_map = decode_ptset_map(&ptset_array);
            InstPointMap {
                refno,
                att_type,
                ptset_map,
            }
        })
        .collect())
}

#[tokio::test]
async fn test_query_bran_children_point_map() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/67947").unwrap();
    let r = query_bran_children_point_map(refno.into()).await.unwrap();
    dbg!(&r);
}

/// 查询参考号对应的点集
pub async fn query_point_map(refno: RefnoEnum) -> anyhow::Result<Option<InstPointMap>> {
    // ptset 现在是数组，需要转换为 BTreeMap<String, CateAxisParam>
    let sql = format!(
        "select id as refno, id->inst_relate.out.ptset as ptset_array, noun as att_type from {};",
        refno.to_pe_key()
    );
    let mut response = SUL_DB.query_response(&sql).await?;
    let Ok(mut result) =
        take_vec::<(RefnoEnum, Option<serde_json::Value>, String)>(&mut response, 0)
    else {
        dbg!(format!("sql 查询出错: {}", sql));
        return Ok(None);
    };
    if result.is_empty() {
        return Ok(None);
    }
    let (refno, ptset_array, att_type) = result.remove(0);
    let ptset_map = decode_ptset_map(&ptset_array);
    Ok(Some(InstPointMap {
        refno,
        att_type,
        ptset_map,
    }))
}

/// 查询多个参考号对应的点集
pub async fn query_refnos_point_map(
    refnos: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefnoEnum, InstPointMap>> {
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    // ptset 现在是数组，需要转换为 BTreeMap<String, CateAxisParam>
    let sql = format!(
        "select id as refno, id->inst_relate.out.ptset as ptset_array, noun as att_type from [{}];",
        refnos.join(",")
    );
    let mut response = SUL_DB.query_response(&sql).await?;
    let Ok(result) = take_vec::<(RefnoEnum, Option<serde_json::Value>, String)>(&mut response, 0)
    else {
        dbg!(format!("sql 查询出错: {}", sql));
        return Ok(HashMap::default());
    };
    Ok(result
        .into_iter()
        .map(|(refno, ptset_array, att_type)| {
            let ptset_map = decode_ptset_map(&ptset_array);
            (
                refno,
                InstPointMap {
                    refno,
                    att_type,
                    ptset_map,
                },
            )
        })
        .collect())
}

///通过geo hash 查询参考号
pub async fn query_refnos_by_geo_hash(id: &str) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!(
        "array::distinct(array::flatten(select value in<-inst_relate.in from inst_geo:⟨{}⟩<-geo_relate));",
        id
    );
    let mut response = SUL_DB.query_response(&sql).await?;
    let result = take_vec::<RefnoEnum>(&mut response, 0)?;
    Ok(result)
}

/// 获取arrive和leave的世界坐标
pub fn get_arrive_leave_info(
    refno: RefU64,
    point_map: &HashMap<RefU64, InstPointMap>,
    attr: &NamedAttrMap,
    transform: Transform,
) -> (Vec3, Vec3) {
    let mut arrive_pos = Vec3::ZERO;
    let mut leave_pos = Vec3::ZERO;
    if let Some(points) = point_map.get(&refno) {
        if let Some(NamedAttrValue::IntegerType(arrive)) = attr.get_val("ARRI") {
            if let Some(point_info) = points.ptset_map.get(&arrive.to_string()) {
                let arrive_point = transform.transform_point(point_info.pt.0);
                arrive_pos = arrive_point;
            }
            if let Some(NamedAttrValue::IntegerType(leave)) = attr.get_val("LEAV") {
                if let Some(point_info) = points.ptset_map.get(&leave.to_string()) {
                    let leave_point = transform.transform_point(point_info.pt.0);
                    leave_pos = leave_point;
                }
            }
        }
    }
    (arrive_pos, leave_pos)
}

#[tokio::test]
async fn test_query_refnos_point_map() -> anyhow::Result<()> {
    init_test_surreal().await;
    let refno = RefnoEnum::from("24383/101165");
    let r = query_refnos_point_map(vec![refno]).await?;
    dbg!(&r);
    let bran_refno = RefnoEnum::from("24383/101155");
    let r = query_bran_children_point_map(bran_refno).await?;
    dbg!(&r);
    let r = query_point_map(refno).await?;
    dbg!(&r);
    Ok(())
}

#[cfg(test)]
mod ptset_decode_tests {
    use super::*;
    use crate::shape::pdms_shape::RsVec3;
    use crate::vec3_pool::CateAxisParamCompact;
    use glam::Vec3;

    #[test]
    fn decode_compressed_ptset() {
        let compact = CateAxisParamCompact {
            n: 1,
            p: Some([1.0, 2.0, 3.0]),
            d: None,
            df: None,
            rd: None,
            b: None,
            c: None,
            w: None,
            h: None,
            r: None,
        };
        let value = serde_json::to_value(vec![compact]).unwrap();
        let map = decode_ptset_map(&Some(value));
        let pt = map.get("1").expect("number 1 should exist").pt.0;
        assert!((pt - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-4);
    }

    #[test]
    fn decode_raw_ptset() {
        let raw_param = CateAxisParam {
            refno: Default::default(),
            number: 2,
            pt: RsVec3(Vec3::new(4.0, 5.0, 6.0)),
            dir: None,
            dir_flag: 1.0,
            ref_dir: None,
            pbore: 0.0,
            pwidth: 0.0,
            pheight: 0.0,
            pconnect: String::new(),
        };
        let value = serde_json::to_value(vec![raw_param]).unwrap();
        let map = decode_ptset_map(&Some(value));
        let pt = map.get("2").expect("number 2 should exist").pt.0;
        assert!((pt - Vec3::new(4.0, 5.0, 6.0)).length() < 1e-4);
    }
}

//query_ptset
/// 查询RefnoEnum对应的点集合
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PtsetResult {
    pub transform: Transform,
    pub points: Vec<Vec3>,
}

/// 查询参考号对应的点集合
///
/// # 参数
/// * `refno` - 需要查询的参考号
///
/// # 返回值
/// * `Ok(Some(PtsetResult))` - 查询成功且找到点集
/// * `Ok(None)` - 查询成功但未找到点集
/// * `Err` - 查询过程中发生错误
///
/// # 实现说明
/// 1. 通过SQL查询inst_relate表中的数据
/// 2. 获取世界坐标变换矩阵(world_trans.d)和点集(ptset)
/// 3. 将结果解析为PtsetResult结构体
pub async fn query_ptset(refno: RefnoEnum) -> anyhow::Result<Option<PtsetResult>> {
    // 构建SQL查询语句:
    // - world_trans.d 获取世界坐标变换矩阵
    // - out.ptset[*].pt 获取点集（ptset 现在是数组，直接提取 pt 字段）
    let sql = format!(
        "(select world_trans.d as transform, out.ptset[*].pt as points from {0})[0]",
        to_table_key!(refno, "inst_relate")
    );
    // 执行查询
    let mut response = SUL_DB.query_response(&sql).await?;
    // 解析查询结果为PtsetResult类型
    let result = take_option::<PtsetResult>(&mut response, 0)?;
    // dbg!(&result);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_surreal;

    #[tokio::test]
    async fn test_query_ptset() -> anyhow::Result<()> {
        // Initialize the test database
        init_test_surreal().await;

        // Create a test RefnoEnum
        let refno = "17496_170587".into();

        // Query the point set
        let result = query_ptset(refno).await?;
        assert!(result.is_some(), "Point set result should not be None");

        let ptset_result = result.unwrap();
        let transform = ptset_result.transform;
        let points = ptset_result.points;

        // Perform assertions
        assert!(!points.is_empty(), "Point set should not be empty");

        // Check if the transform is valid
        assert!(transform.is_finite(), "Transform should be finite");

        // Check if all points are valid Vec3
        for point in &points {
            assert!(
                point.x.is_finite() && point.y.is_finite() && point.z.is_finite(),
                "All components of the point should be finite"
            );
        }

        // You might want to add more specific assertions based on expected data
        // For example, checking for a specific number of points or specific point values

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_loops_and_height() -> anyhow::Result<()> {
        // Initialize the test database
        init_test_surreal().await?;

        // 使用已知的 PANE 元素进行测试: 24381_36716
        // 结构: PANE(24381_36716) -> PLOO(24381_36717) -> PAVE(多个顶点)
        let refno: RefnoEnum = "24381_36716".into();

        // 查询 loops 和 height
        let result = fetch_loops_and_height(refno).await?;

        // 验证返回的 loops 不为空
        assert!(!result.loops.is_empty(), "Loops should not be empty");

        // 验证高度值有效（已知该 PANE 的高度为 3050.0）
        assert!(result.height > 0.0, "Height should be greater than 0");
        assert!(
            (result.height - 3050.0).abs() < 1.0,
            "Height should be approximately 3050.0, got {}",
            result.height
        );

        // 验证第一个 loop 有多个顶点（已知该 PLOO 有 9 个 PAVE 子元素）
        let first_loop = &result.loops[0];
        assert!(
            first_loop.len() >= 3,
            "First loop should have at least 3 vertices, got {}",
            first_loop.len()
        );
        assert_eq!(
            first_loop.len(),
            9,
            "First loop should have 9 vertices (PAVE elements)"
        );

        // 验证顶点数据有效
        for point in first_loop {
            assert!(
                point.x.is_finite() && point.y.is_finite() && point.z.is_finite(),
                "All components of the point should be finite"
            );
        }

        println!("fetch_loops_and_height test passed:");
        println!("  - Number of loops: {}", result.loops.len());
        println!("  - First loop vertices: {}", first_loop.len());
        println!("  - Height: {}", result.height);

        Ok(())
    }
}
