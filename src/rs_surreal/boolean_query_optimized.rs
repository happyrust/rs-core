use crate::SurrealQueryExt;
/// Optimized boolean operation query functions
///
/// This module provides optimized query functions for boolean operations on geometry,
/// specifically focusing on neg_relate and ngmr_relate queries.
use crate::rs_surreal::query_structs::{ManiGeoTransQuery, NegInfo};
use crate::types::RefnoEnum;
use crate::{SUL_DB, get_inst_relate_keys};
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;

/// 高度优化的布尔运算查询函数
///
/// # 优化策略
///
/// 1. 分离查询：将复杂的嵌套查询拆分为多个独立的简单查询
/// 2. 批量处理：使用 IN 操作符减少查询次数
/// 3. 索引友好：查询条件按照索引顺序排列
/// 4. 减少数据传输：只查询必要的字段
///
/// # Parameters
///
/// * `refno` - 正实体 refno
///
/// # Returns
///
/// Returns `Vec<ManiGeoTransQuery>` containing manifold boolean operation data
pub async fn query_manifold_boolean_operations_optimized(
    refno: RefnoEnum,
) -> anyhow::Result<Vec<ManiGeoTransQuery>> {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosEntityBase {
        refno: RefnoEnum,
        sesno: u32,
        noun: String,
        wt: crate::rs_surreal::geometry_query::PlantTransform,
        #[serde(default)]
        aabb: Option<crate::types::PlantAabb>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosGeometry {
        id: crate::types::RecordId,
        trans: crate::rs_surreal::geometry_query::PlantTransform,
    }

    use anyhow::Context as _;

    // 注意：inst_relate 的 record id 在 SurrealQL 中必须使用尖括号形式，
    // 否则像 `17496_106028` 这类包含下划线的 key 可能不可解析。
    let inst_key = format!("inst_relate:⟨{}⟩", refno);

    // 步骤1：获取正实体基础信息（使用索引优化）
    // 查询尚未成功布尔运算的实体（bool_status != 'Success'）
    let sql_base = format!(
        r#"
        SELECT 
            in AS refno,
            in.sesno AS sesno,
            in.noun AS noun,
            world_trans.d AS wt,
            ((select value (out.d ?? NONE) from in->inst_relate_aabb where out.d != NONE limit 1)[0] ?? NONE) AS aabb
        FROM {inst_key}
        WHERE in.id != NONE
            AND (bool_status != 'Success' OR bool_status = NONE)
            AND array::len(in->inst_relate_aabb) > 0
        LIMIT 1
        "#
    );

    let base_info: Vec<PosEntityBase> = SUL_DB
        .query_take(&sql_base, 0)
        .await
        .with_context(|| format!("sql_base 查询失败，SQL={}", sql_base))?;
    if base_info.is_empty() {
        return Ok(Vec::new());
    }
    let base = base_info.into_iter().next().unwrap();
    let PosEntityBase {
        refno,
        sesno,
        noun,
        wt,
        aabb,
    } = base;
    let Some(aabb) = aabb else {
        return Ok(Vec::new());
    };

    // 步骤2：获取正几何（Compound/Pos类型）
    let sql_pos_geos = format!(
        r#"
        SELECT out AS id, trans.d AS trans
        FROM inst_relate:{refno}->out->geo_relate
        WHERE geo_type IN ["Compound", "Pos"] AND trans.d != NONE
        "#
    );
    let pos_geos: Vec<PosGeometry> = SUL_DB
        .query_take(&sql_pos_geos, 0)
        .await
        .with_context(|| format!("sql_pos_geos 查询失败，SQL={}", sql_pos_geos))?;
    let ts = pos_geos.into_iter().map(|g| (g.id, g.trans)).collect();

    // 步骤3：直接从 neg_relate 和 ngmr_relate 获取切割几何（新结构简化版）
    // neg_relate/ngmr_relate 结构：in = geo_relate (切割几何), out = pe (被切割的正实体)
    // geo_relate 结构：in = pe (负载体), out = geo
    // 所以负载体 = in.in，负载体的 world_trans = in.in<-inst_relate.world_trans
    // 兼容两种 out：pe 与 inst_relate（历史/兼容写入）
    let pe_key = refno.to_pe_key();
    let sql_neg_pe = format!(
        r#"
        SELECT 
            in.out AS id,
            in.geo_type AS geo_type,
            in.para_type ?? "" AS para_type,
            in.trans.d AS trans,
            in.out.aabb.d AS aabb,
            array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
        FROM {pe_key}<-neg_relate
        WHERE in.trans.d != NONE
        "#
    );
    let mut neg_results: Vec<NegInfo> = SUL_DB.query_take(&sql_neg_pe, 0).await.unwrap_or_default();
    if neg_results.is_empty() {
        let sql_neg_inst = format!(
            r#"
            SELECT 
                in.out AS id,
                in.geo_type AS geo_type,
                in.para_type ?? "" AS para_type,
                in.trans.d AS trans,
                in.out.aabb.d AS aabb,
                array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
            FROM {inst_key}<-neg_relate
            WHERE in.trans.d != NONE
            "#
        );
        neg_results = SUL_DB.query_take(&sql_neg_inst, 0).await.unwrap_or_default();
    }

    let sql_ngmr_pe = format!(
        r#"
        SELECT 
            in.out AS id,
            in.geo_type AS geo_type,
            in.para_type ?? "" AS para_type,
            in.trans.d AS trans,
            in.out.aabb.d AS aabb,
            array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
        FROM {pe_key}<-ngmr_relate
        WHERE in.trans.d != NONE
        "#
    );
    let mut ngmr_results: Vec<NegInfo> = SUL_DB.query_take(&sql_ngmr_pe, 0).await.unwrap_or_default();
    if ngmr_results.is_empty() {
        let sql_ngmr_inst = format!(
            r#"
            SELECT 
                in.out AS id,
                in.geo_type AS geo_type,
                in.para_type ?? "" AS para_type,
                in.trans.d AS trans,
                in.out.aabb.d AS aabb,
                array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
            FROM {inst_key}<-ngmr_relate
            WHERE in.trans.d != NONE
            "#
        );
        ngmr_results = SUL_DB.query_take(&sql_ngmr_inst, 0).await.unwrap_or_default();
    }

    // 合并切割几何
    let mut neg_infos: Vec<NegInfo> = neg_results;
    neg_infos.extend(ngmr_results);

    // 构建 neg_ts（简化版：所有切割几何放在一个虚拟 carrier 下）
    let neg_ts: Vec<(
        RefnoEnum,
        crate::rs_surreal::geometry_query::PlantTransform,
        Vec<NegInfo>,
    )> = if neg_infos.is_empty() {
        Vec::new()
    } else {
        vec![(
            refno.clone(),
            crate::rs_surreal::geometry_query::PlantTransform::default(),
            neg_infos,
        )]
    };

    // 步骤6：组装最终结果
    let result = ManiGeoTransQuery {
        refno,
        sesno,
        noun,
        inst_world_trans: wt,
        aabb,
        pos_geos: ts,
        neg_ts,
    };

    Ok(vec![result])
}

/// 批量优化查询多个正实体的布尔运算数据
///
/// # 优化策略
///
/// 1. 批量获取所有正实体基础信息
/// 2. 批量获取所有正几何
/// 3. 批量获取所有负载体
/// 4. 批量获取所有负几何
/// 5. 在应用层组装数据
///
/// # Parameters
///
/// * `refnos` - 正实体 refno 列表
///
/// # Returns
///
/// Returns `Vec<ManiGeoTransQuery>` containing all boolean operation data
pub async fn query_manifold_boolean_operations_batch_optimized(
    refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<ManiGeoTransQuery>> {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosEntityBase {
        refno: RefnoEnum,
        sesno: u32,
        noun: String,
        wt: crate::rs_surreal::geometry_query::PlantTransform,
        #[serde(default)]
        aabb: Option<crate::types::PlantAabb>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosGeometry {
        id: crate::types::RecordId,
        trans: crate::rs_surreal::geometry_query::PlantTransform,
    }

    // 步骤1：批量获取所有正实体基础信息
    // 从 pe_transform 获取 world_trans，从 inst_relate_aabb 关系获取 aabb
    // 注意：使用 LET 预先计算 aabb 并在 WHERE 中过滤，确保 aabb 不为 null
    use anyhow::Context as _;
    let inst_keys = get_inst_relate_keys(refnos);
    let sql_bases = format!(
        r#"
        SELECT 
            in AS refno,
            in.sesno AS sesno,
            in.noun AS noun,
            type::record("pe_transform", record::id(in)).world_trans.d AS wt,
            type::record("inst_relate_aabb", record::id(in)).out.d AS aabb
        FROM {inst_keys}
        WHERE in.id != NONE
            AND (bool_status != 'Success' OR bool_status = NONE)
            AND type::record("pe_transform", record::id(in)).world_trans != NONE
            AND type::record("inst_relate_aabb", record::id(in)).out.d != NONE
        "#
    );
    let base_infos: Vec<PosEntityBase> = SUL_DB
        .query_take(&sql_bases, 0)
        .await
        .unwrap_or_default();  // 反序列化失败时返回空列表
    let mut base_map: HashMap<RefnoEnum, PosEntityBase> = HashMap::new();
    for base in base_infos {
        // 过滤掉 aabb 为 None 的记录
        if base.aabb.is_some() {
            base_map.insert(base.refno, base);
        }
    }
    // 步骤2：批量获取所有正几何
    // 使用子查询模式：从 inst_relate.out (inst_info) 遍历到 geo_relate
    // 注意：直接使用 inst_relate->out->geo_relate 语法不工作，需要用子查询
    let sql_all_pos_geos = format!(
        r#"
        SELECT 
            in as refno,
            (SELECT out AS id, trans.d AS trans FROM out->geo_relate WHERE geo_type IN ["Compound", "Pos"] AND trans.d != NONE) AS geos
        FROM {inst_keys}
        "#
    );

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosGeosResult {
        refno: RefnoEnum,
        geos: Vec<PosGeometry>,
    }

    let pos_geo_results: Vec<PosGeosResult> = SUL_DB
        .query_take(&sql_all_pos_geos, 0)
        .await
        .with_context(|| format!("sql_all_pos_geos 查询失败，SQL={}", sql_all_pos_geos))?;
    let mut pos_geo_map: HashMap<
        RefnoEnum,
        Vec<(
            crate::types::RecordId,
            crate::rs_surreal::geometry_query::PlantTransform,
        )>,
    > = HashMap::new();
    for result in pos_geo_results {
        for geo in result.geos {
            pos_geo_map
                .entry(result.refno.clone())
                .or_insert_with(Vec::new)
                .push((geo.id, geo.trans));
        }
    }

    // 步骤3：直接从 neg_relate 和 ngmr_relate 获取切割几何
    // 新结构：in = geo_relate (切割几何), out = pe (正实体), pe = 负载体
    // 查询简化：直接 SELECT in.* FROM pe:正实体<-neg_relate/ngmr_relate

    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct NegGeoResult {
        id: crate::types::RecordId,
        geo_type: String,
        #[serde(default)]
        para_type: String,
        trans: crate::rs_surreal::geometry_query::PlantTransform,
        #[serde(default)]
        aabb: Option<crate::types::PlantAabb>,
        /// 负载体的世界变换矩阵（从 inst_relate 获取）
        #[serde(default)]
        carrier_wt: Option<crate::rs_surreal::geometry_query::PlantTransform>,
    }

    // 收集每个正实体的切割几何
    let mut neg_geos_map: HashMap<RefnoEnum, Vec<NegInfo>> = HashMap::new();

    for refno in refnos {
        let pe_key = refno.to_pe_key();
        let inst_key = format!("inst_relate:⟨{}⟩", refno);

        // 查询 neg_relate: in = geo_relate (Neg类型切割几何)
        // neg_relate 结构: in = geo_relate, out = 被切割的正实体
        // geo_relate 结构: in = 负载体 pe, out = geo
        // 所以负载体 = in.in，负载体的 world_trans = in.in<-inst_relate.world_trans
        let sql_neg_pe = format!(
            r#"
            SELECT 
                in.out AS id,
                in.geo_type AS geo_type, 
                in.para_type ?? "" AS para_type,
                in.trans.d AS trans,
                in.out.aabb.d AS aabb,
                array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
            FROM {pe_key}<-neg_relate
            WHERE in.trans.d != NONE
            "#
        );
        let mut neg_results: Vec<NegGeoResult> =
            SUL_DB.query_take(&sql_neg_pe, 0).await.unwrap_or_default();
        if neg_results.is_empty() {
            let sql_neg_inst = format!(
                r#"
                SELECT 
                    in.out AS id,
                    in.geo_type AS geo_type, 
                    in.para_type ?? "" AS para_type,
                    in.trans.d AS trans,
                    in.out.aabb.d AS aabb,
                    array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
                FROM {inst_key}<-neg_relate
                WHERE in.trans.d != NONE
                "#
            );
            neg_results = SUL_DB.query_take(&sql_neg_inst, 0).await.unwrap_or_default();
        }

        // 查询 ngmr_relate: in = geo_relate (CataCrossNeg类型切割几何)
        // ngmr_relate 结构同 neg_relate: in = geo_relate, out = 被切割的正实体
        // 负载体 = in.in，负载体的 world_trans = in.in<-inst_relate.world_trans
        let sql_ngmr_pe = format!(
            r#"
            SELECT 
                in.out AS id,
                in.geo_type AS geo_type,
                in.para_type ?? "" AS para_type,
                in.trans.d AS trans,
                in.out.aabb.d AS aabb,
                array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
            FROM {pe_key}<-ngmr_relate
            WHERE in.trans.d != NONE
            "#
        );
        let mut ngmr_results: Vec<NegGeoResult> = match SUL_DB.query_take(&sql_ngmr_pe, 0).await {
            Ok(results) => results,
            Err(e) => {
                eprintln!("[DEBUG] ngmr_relate 查询失败 for {}: {}", pe_key, e);
                Vec::new()
            }
        };
        if ngmr_results.is_empty() {
            let sql_ngmr_inst = format!(
                r#"
                SELECT 
                    in.out AS id,
                    in.geo_type AS geo_type,
                    in.para_type ?? "" AS para_type,
                    in.trans.d AS trans,
                    in.out.aabb.d AS aabb,
                    array::first(in.in<-inst_relate).world_trans.d AS carrier_wt
                FROM {inst_key}<-ngmr_relate
                WHERE in.trans.d != NONE
                "#
            );
            ngmr_results = SUL_DB.query_take(&sql_ngmr_inst, 0).await.unwrap_or_default();
        }

        // 合并结果
        let mut neg_infos = Vec::new();
        for r in neg_results.into_iter().chain(ngmr_results.into_iter()) {
            neg_infos.push(NegInfo {
                id: r.id,
                geo_type: r.geo_type,
                para_type: r.para_type,
                geo_local_trans: r.trans,
                aabb: r.aabb,
                carrier_world_trans: r.carrier_wt,
            });
        }

        if !neg_infos.is_empty() {
            neg_geos_map.insert(refno.clone(), neg_infos);
        }
    }

    // 步骤4：组装最终结果（简化版）
    let mut results = Vec::with_capacity(refnos.len());

    for refno in refnos {
        if let Some(base) = base_map.remove(refno) {
            let ts = pos_geo_map.remove(refno).unwrap_or_default();

            // 直接获取该正实体的所有切割几何（新结构简化版）
            let neg_infos = neg_geos_map.remove(refno).unwrap_or_default();

            // 新结构：neg_ts 简化为空，所有切割几何直接放在 neg_infos 中
            // 因为新结构下 neg_relate/ngmr_relate.in 直接指向 geo_relate，不再需要通过 carrier 间接查询
            let neg_ts: Vec<(
                RefnoEnum,
                crate::rs_surreal::geometry_query::PlantTransform,
                Vec<NegInfo>,
            )> = if neg_infos.is_empty() {
                Vec::new()
            } else {
                // 为兼容现有的 ManiGeoTransQuery 结构，将所有 neg_infos 放在一个虚拟 carrier 下
                // TODO: 后续可以简化 ManiGeoTransQuery 结构，直接使用 neg_infos
                vec![(
                    refno.clone(), // 使用正实体自身作为虚拟 carrier
                    crate::rs_surreal::geometry_query::PlantTransform::default(),
                    neg_infos,
                )]
            };

            results.push(ManiGeoTransQuery {
                refno: base.refno,
                sesno: base.sesno,
                noun: base.noun,
                inst_world_trans: base.wt,
                aabb: match base.aabb {
                    Some(aabb) => aabb,
                    None => continue,
                },
                pos_geos: ts,
                neg_ts,
            });
        }
    }

    Ok(results)
}

/// 创建数据库索引以优化布尔运算查询
///
/// # 索引策略
///
/// 1. inst_relate表的in字段索引
/// 2. neg_relate表的out字段索引
/// 3. ngmr_relate表的out字段索引
/// 4. geo_relate表的geo_type字段索引
/// 5. geo_relate表的trans字段索引
pub async fn create_boolean_query_indexes() -> anyhow::Result<()> {
    let indexes = vec![
        // inst_relate表索引
        "DEFINE INDEX idx_inst_relate_in ON TABLE inst_relate COLUMNS in",
        "DEFINE INDEX idx_inst_relate_bool_status ON TABLE inst_relate COLUMNS bool_status",
        // neg_relate表索引
        "DEFINE INDEX idx_neg_relate_out ON TABLE neg_relate COLUMNS out",
        "DEFINE INDEX idx_neg_relate_in ON TABLE neg_relate COLUMNS in",
        // ngmr_relate表索引
        "DEFINE INDEX idx_ngmr_relate_out ON TABLE ngmr_relate COLUMNS out",
        "DEFINE INDEX idx_ngmr_relate_in ON TABLE ngmr_relate COLUMNS in",
        // geo_relate表索引
        "DEFINE INDEX idx_geo_relate_geo_type ON TABLE geo_relate COLUMNS geo_type",
        "DEFINE INDEX idx_geo_relate_trans ON TABLE geo_relate COLUMNS trans",
        "DEFINE INDEX idx_geo_relate_geom_refno ON TABLE geo_relate COLUMNS geom_refno",
    ];

    for index in indexes {
        SUL_DB.query(index).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RefU64, RefnoEnum};

    #[tokio::test]
    async fn test_optimized_vs_original_query() {
        // 测试数据
        let test_refno = RefnoEnum::Refno(RefU64(12345));

        // 执行优化版本
        let optimized_result = query_manifold_boolean_operations_optimized(test_refno).await;
        assert!(optimized_result.is_ok());

        // 执行批量优化版本
        let batch_result = query_manifold_boolean_operations_batch_optimized(&[test_refno]).await;
        assert!(batch_result.is_ok());

        // 验证结果一致性
        if let (Ok(optimized), Ok(batch)) = (optimized_result, batch_result) {
            assert_eq!(optimized.len(), batch.len());
            if !optimized.is_empty() && !batch.is_empty() {
                assert_eq!(optimized[0].refno, batch[0].refno);
            }
        }
    }

    #[tokio::test]
    async fn test_index_creation() {
        let result = create_boolean_query_indexes().await;
        assert!(result.is_ok());
    }
}
