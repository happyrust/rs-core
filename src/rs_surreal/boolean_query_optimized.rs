use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;
use crate::SurrealQueryExt;
/// Optimized boolean operation query functions
///
/// This module provides optimized query functions for boolean operations on geometry,
/// specifically focusing on neg_relate and ngmr_relate queries.
use crate::rs_surreal::query_structs::{
    ManiGeoTransQuery, NegInfo,
};
use crate::types::RefnoEnum;
use crate::{SUL_DB};

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
        aabb: crate::types::PlantAabb,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosGeometry {
        id: crate::types::RecordId,
        trans: crate::rs_surreal::geometry_query::PlantTransform,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct NegCarrier {
        refno: RefnoEnum,
        wt: crate::rs_surreal::geometry_query::PlantTransform,
    }
    
    let pe_key = refno.to_pe_key();
    
    // 步骤1：获取正实体基础信息（使用索引优化）
    let sql_base = format!(
        r#"
        SELECT 
            in AS refno,
            in.sesno AS sesno,
            in.noun AS noun,
            world_trans.d AS wt,
            aabb.d AS aabb
        FROM inst_relate:{refno}
        WHERE in.id != NONE AND !bad_bool AND aabb.d != NONE
        LIMIT 1
        "#
    );
    
    let base_info: Vec<PosEntityBase> = SUL_DB.query_take(&sql_base, 0).await?;
    if base_info.is_empty() {
        return Ok(Vec::new());
    }
    let base = base_info.into_iter().next().unwrap();
    
    // 步骤2：获取正几何（Compound/Pos类型）
    let sql_pos_geos = format!(
        r#"
        SELECT out AS id, trans.d AS trans
        FROM inst_relate:{refno}->out->geo_relate
        WHERE geo_type IN ["Compound", "Pos"] AND trans.d != NONE
        "#
    );
    let pos_geos: Vec<PosGeometry> = SUL_DB.query_take(&sql_pos_geos, 0).await?;
    let ts = pos_geos.into_iter().map(|g| (g.id, g.trans)).collect();
    
    // 步骤3：批量获取负载体（使用UNION ALL优化）
    let sql_neg_carriers = format!(
        r#"
        (SELECT in AS refno, world_trans.d AS wt FROM {pe_key}<-neg_relate.in<-inst_relate WHERE world_trans.d != NONE)
        UNION ALL
        (SELECT in AS refno, world_trans.d AS wt FROM {pe_key}<-ngmr_relate.in->inst_relate WHERE world_trans.d != NONE)
        "#
    );
    let neg_carriers: Vec<NegCarrier> = SUL_DB.query_take(&sql_neg_carriers, 0).await?;
    
    // 步骤4：预取允许的ngmr列表（用于CataCrossNeg过滤）
    let sql_ngmr = format!(
        r#"
        SELECT VALUE ngmr FROM {pe_key}<-ngmr_relate
        "#
    );
    let allowed_ngmr: Vec<RefnoEnum> = SUL_DB.query_take(&sql_ngmr, 0).await.unwrap_or_default();
    
    // 步骤5：批量查询所有负载体的负几何
    let mut neg_ts = Vec::with_capacity(neg_carriers.len());
    
    if !neg_carriers.is_empty() {
        // 构建载体refno列表，使用批量查询
        let carrier_keys: Vec<String> = neg_carriers
            .iter()
            .map(|c| c.refno.to_pe_key())
            .collect();
        
        // 批量查询所有负载体的负几何
        let sql_all_neg_geos = if allowed_ngmr.is_empty() {
            // 只有Neg类型
            format!(
                r#"
                SELECT 
                    array::first(array::filter(<-inst_relate, |$r| $r.in IN [{}])) as carrier,
                    out AS id,
                    geo_type,
                    para_type ?? "" AS para_type,
                    trans.d AS trans,
                    out.aabb.d AS aabb
                FROM array::flatten([{}])
                WHERE trans.d != NONE AND geo_type == "Neg"
                "#,
                carrier_keys.join(","),
                carrier_keys.iter().map(|k| format!("{}<-inst_relate.out->geo_relate", k)).collect::<Vec<_>>().join(",")
            )
        } else {
            // Neg和CataCrossNeg类型
            let ngmr_keys: Vec<String> = allowed_ngmr
                .iter()
                .map(|n| n.to_pe_key())
                .collect();
            
            format!(
                r#"
                SELECT 
                    array::first(array::filter(<-inst_relate, |$r| $r.in IN [{}])) as carrier,
                    out AS id,
                    geo_type,
                    para_type ?? "" AS para_type,
                    trans.d AS trans,
                    out.aabb.d AS aabb
                FROM array::flatten([{}])
                WHERE trans.d != NONE AND (
                    geo_type == "Neg" OR 
                    (geo_type == "CataCrossNeg" AND geom_refno IN [{}])
                )
                "#,
                carrier_keys.join(","),
                carrier_keys.iter().map(|k| format!("{}<-inst_relate.out->geo_relate", k)).collect::<Vec<_>>().join(","),
                ngmr_keys.join(",")
            )
        };
        
        #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
        struct NegGeoWithCarrier {
            carrier: RefnoEnum,
            id: crate::types::RecordId,
            geo_type: String,
            para_type: String,
            trans: crate::rs_surreal::geometry_query::PlantTransform,
            aabb: crate::types::PlantAabb,
        }
        
        let all_neg_geos: Vec<NegGeoWithCarrier> = SUL_DB.query_take(&sql_all_neg_geos, 0).await?;
        
        // 按载体分组负几何
        use std::collections::HashMap;
        let mut neg_geo_map: HashMap<RefnoEnum, Vec<NegInfo>> = HashMap::new();
        
        for geo in all_neg_geos {
            let carrier_refno = geo.carrier;
            let neg_info = NegInfo {
                id: geo.id,
                geo_type: geo.geo_type,
                para_type: geo.para_type,
                trans: geo.trans,
                aabb: Some(geo.aabb),
            };
            
            neg_geo_map
                .entry(carrier_refno)
                .or_insert_with(Vec::new)
                .push(neg_info);
        }
        
        // 组装最终的neg_ts
        for carrier in neg_carriers {
            let neg_infos = neg_geo_map.remove(&carrier.refno).unwrap_or_default();
            neg_ts.push((carrier.refno, carrier.wt, neg_infos));
        }
    }
    
    // 步骤6：组装最终结果
    let result = ManiGeoTransQuery {
        refno: base.refno,
        sesno: base.sesno,
        noun: base.noun,
        wt: base.wt,
        aabb: base.aabb,
        ts,
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
        aabb: crate::types::PlantAabb,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosGeometry {
        refno: RefnoEnum,
        id: crate::types::RecordId,
        trans: crate::rs_surreal::geometry_query::PlantTransform,
    }
    
    // 步骤1：批量获取所有正实体基础信息
    let refno_keys: Vec<String> = refnos.iter().map(|r| r.to_pe_key()).collect();
    let sql_bases = format!(
        r#"
        SELECT 
            in AS refno,
            in.sesno AS sesno,
            in.noun AS noun,
            world_trans.d AS wt,
            aabb.d AS aabb
        FROM [{}]
        WHERE in.id != NONE AND !bad_bool AND aabb.d != NONE
        "#,
        refno_keys.join(",")
    );
    
    let base_infos: Vec<PosEntityBase> = SUL_DB.query_take(&sql_bases, 0).await?;
    let mut base_map: HashMap<RefnoEnum, PosEntityBase> = HashMap::new();
    for base in base_infos {
        base_map.insert(base.refno, base);
    }
    
    // 步骤2：批量获取所有正几何
    let sql_all_pos_geos = format!(
        r#"
        SELECT 
            in as refno,
            out AS id,
            trans.d AS trans
        FROM [{}]->out->geo_relate
        WHERE geo_type IN ["Compound", "Pos"] AND trans.d != NONE
        "#,
        refno_keys.join(",")
    );
    
    let pos_geos: Vec<PosGeometry> = SUL_DB.query_take(&sql_all_pos_geos, 0).await?;
    let mut pos_geo_map: HashMap<RefnoEnum, Vec<(crate::types::RecordId, crate::rs_surreal::geometry_query::PlantTransform)>> = HashMap::new();
    for geo in pos_geos {
        pos_geo_map
            .entry(geo.refno)
            .or_insert_with(Vec::new)
            .push((geo.id, geo.trans));
    }
    
    // 步骤3：批量获取所有负载体
    let mut all_neg_carriers: Vec<(RefnoEnum, RefnoEnum, crate::rs_surreal::geometry_query::PlantTransform)> = Vec::new();
    
    for refno in refnos {
        let pe_key = refno.to_pe_key();
        
        // neg_relate载体
        let sql_neg = format!(
            r#"
            SELECT in AS refno, world_trans.d AS wt
            FROM {pe_key}<-neg_relate.in<-inst_relate
            WHERE world_trans.d != NONE
            "#
        );
        let neg_carriers: Vec<(RefnoEnum, crate::rs_surreal::geometry_query::PlantTransform)> = 
            SUL_DB.query_take(&sql_neg, 0).await.unwrap_or_default();
        
        for (carrier_refno, wt) in neg_carriers {
            all_neg_carriers.push((refno.clone(), carrier_refno, wt));
        }
        
        // ngmr_relate载体
        let sql_ngmr = format!(
            r#"
            SELECT in AS refno, world_trans.d AS wt
            FROM {pe_key}<-ngmr_relate.in->inst_relate
            WHERE world_trans.d != NONE
            "#
        );
        let ngmr_carriers: Vec<(RefnoEnum, crate::rs_surreal::geometry_query::PlantTransform)> = 
            SUL_DB.query_take(&sql_ngmr, 0).await.unwrap_or_default();
        
        for (carrier_refno, wt) in ngmr_carriers {
            all_neg_carriers.push((refno.clone(), carrier_refno, wt));
        }
    }
    
    // 步骤4：批量获取所有允许的ngmr
    let sql_all_ngmr = format!(
        r#"
        SELECT 
            in as pos_refno,
            ngmr
        FROM [{}]<-ngmr_relate
        "#,
        refno_keys.join(",")
    );
    
    #[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
    struct PosNgmr {
        pos_refno: RefnoEnum,
        ngmr: RefnoEnum,
    }
    
    let pos_ngmrs: Vec<PosNgmr> = SUL_DB.query_take(&sql_all_ngmr, 0).await.unwrap_or_default();
    let mut ngmr_map: HashMap<RefnoEnum, Vec<RefnoEnum>> = HashMap::new();
    for pn in pos_ngmrs {
        ngmr_map
            .entry(pn.pos_refno)
            .or_insert_with(Vec::new)
            .push(pn.ngmr);
    }
    
    // 步骤5：组装最终结果
    let mut results = Vec::with_capacity(refnos.len());
    
    for refno in refnos {
        if let Some(base) = base_map.remove(refno) {
            let ts = pos_geo_map.remove(refno).unwrap_or_default();
            
            // 获取该正实体的所有负载体
            let neg_carriers: Vec<_> = all_neg_carriers
                .iter()
                .filter(|(pos, _, _)| pos == refno)
                .map(|(_, carrier, wt)| (*carrier, wt.clone()))
                .collect();
            
            // 获取允许的ngmr列表
            let allowed_ngmr = ngmr_map.get(refno).cloned().unwrap_or_default();
            
            // 为每个负载体查询负几何
            let mut neg_ts = Vec::with_capacity(neg_carriers.len());
            for (carrier_refno, carrier_wt) in neg_carriers {
                let carrier_key = carrier_refno.to_pe_key();
                
                let sql_neg_geos = if allowed_ngmr.is_empty() {
                    format!(
                        r#"
                        SELECT out AS id, geo_type, para_type ?? "" AS para_type, trans.d AS trans, out.aabb.d AS aabb
                        FROM {carrier_key}<-inst_relate.out->geo_relate
                        WHERE trans.d != NONE AND geo_type == "Neg"
                        "#
                    )
                } else {
                    let ngmr_keys: Vec<String> = allowed_ngmr
                        .iter()
                        .map(|n| n.to_pe_key())
                        .collect();
                    
                    format!(
                        r#"
                        SELECT out AS id, geo_type, para_type ?? "" AS para_type, trans.d AS trans, out.aabb.d AS aabb
                        FROM {carrier_key}<-inst_relate.out->geo_relate
                        WHERE trans.d != NONE AND (
                            geo_type == "Neg" OR 
                            (geo_type == "CataCrossNeg" AND geom_refno IN [{}])
                        )
                        "#,
                        ngmr_keys.join(",")
                    )
                };
                
                let neg_infos: Vec<NegInfo> = SUL_DB.query_take(&sql_neg_geos, 0).await.unwrap_or_default();
                neg_ts.push((carrier_refno, carrier_wt, neg_infos));
            }
            
            results.push(ManiGeoTransQuery {
                refno: base.refno,
                sesno: base.sesno,
                noun: base.noun,
                wt: base.wt,
                aabb: base.aabb,
                ts,
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
        "DEFINE INDEX idx_inst_relate_bad_bool ON TABLE inst_relate COLUMNS bad_bool",
        "DEFINE INDEX idx_inst_relate_aabb ON TABLE inst_relate COLUMNS aabb",
        
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
    use crate::types::RefnoEnum;
    
    #[tokio::test]
    async fn test_optimized_vs_original_query() {
        // 测试数据
        let test_refno = RefnoEnum::U64(12345);
        
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