//! XKT 生成相关的 SurrealDB 查询函数
//!
//! 该模块提供了从 SurrealDB 查询几何体数据用于 XKT 文件生成的功能

use crate::{RefnoEnum, SUL_DB, SurlValue, get_inst_relate_keys};
use bevy_transform::components::Transform;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::collections::HashMap;

/// XKT 几何体查询结果
///
/// 包含生成 XKT 文件所需的所有几何体信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XktGeometryData {
    /// 参考号
    pub refno: RefnoEnum,
    /// 几何体哈希值（用于查找 .mesh 文件）
    pub geo_hash: String,
    /// 世界变换矩阵
    pub world_trans: Transform,
    /// 是否已生成 mesh
    pub meshed: bool,
    /// 构件类型（用于颜色分配）
    pub noun: String,
    /// 局部变换（从几何到实例）
    #[serde(default)]
    pub local_trans: Option<Transform>,
}

/// 查询指定参考号的所有几何体数据
///
/// # 参数
/// * `refnos` - 参考号列表
///
/// # 返回
/// 返回几何体数据列表，每个几何体包含：
/// - refno: 参考号
/// - geo_hash: 几何体哈希（对应 .mesh 文件名）
/// - world_trans: 世界变换矩阵
/// - meshed: 是否已生成 mesh
/// - noun: 构件类型
/// - local_trans: 局部变换（可选）
///
/// # 示例
/// ```no_run
/// use aios_core::{RefnoEnum, query_xkt_geometries};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let refnos = vec!["17496/266203".parse()?];
///     let geometries = query_xkt_geometries(&refnos).await?;
///     println!("找到 {} 个几何体", geometries.len());
///     Ok(())
/// }
/// ```
pub async fn query_xkt_geometries(
    refnos: &[RefnoEnum],
) -> Result<Vec<XktGeometryData>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let inst_keys = get_inst_relate_keys(refnos);
    
    // 查询 SQL:
    // 1. 从 inst_relate 获取世界变换
    // 2. 通过 geo_relate 获取几何体信息
    // 3. 只选择已生成 mesh 且非 bad 的几何体
    let sql = format!(
        r#"
        SELECT 
            in as refno,
            in.noun as noun,
            world_trans.d as world_trans,
            record::id(out) as geo_hash,
            out.meshed as meshed,
            trans.d as local_trans
        FROM {inst_keys}->geo_relate
        WHERE 
            world_trans.d != none 
            AND out.meshed = true 
            AND !out.bad
            AND out.id != none
        "#
    );
    
    let mut response = SUL_DB.query(sql).await
        .context("查询 XKT 几何体数据失败")?;
    
    let values: Vec<SurlValue> = response.take(0)
        .context("解析查询结果失败")?;
    
    let geometries: Vec<XktGeometryData> = values
        .into_iter()
        .filter_map(|v| {
            serde_json::from_value(v.into_json_value()).ok()
        })
        .collect();
    
    Ok(geometries)
}

/// 检查 mesh 文件状态
///
/// 从数据库中检查指定的几何体哈希是否已生成 mesh
///
/// # 参数
/// * `geo_hashes` - 几何体哈希列表
///
/// # 返回
/// 返回 (已生成的哈希列表, 缺失的哈希列表)
///
/// # 示例
/// ```no_run
/// use aios_core::check_mesh_status;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let hashes = vec!["12345".to_string(), "67890".to_string()];
///     let (found, missing) = check_mesh_status(&hashes).await?;
///     println!("找到: {}, 缺失: {}", found.len(), missing.len());
///     Ok(())
/// }
/// ```
pub async fn check_mesh_status(
    geo_hashes: &[String],
) -> Result<(Vec<String>, Vec<String>)> {
    if geo_hashes.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let hashes_str = geo_hashes
        .iter()
        .map(|h| format!("inst_geo:⟨{}⟩", h))
        .collect::<Vec<_>>()
        .join(",");
    
    let sql = format!(
        r#"
        SELECT 
            record::id(id) as geo_hash,
            meshed
        FROM [{}]
        WHERE meshed = true
        "#,
        hashes_str
    );
    
    let mut response = SUL_DB.query(sql).await
        .context("查询 mesh 状态失败")?;
    
    let values: Vec<SurlValue> = response.take(0)
        .context("解析 mesh 状态失败")?;
    
    let found: Vec<String> = values
        .into_iter()
        .filter_map(|v| {
            let json = v.into_json_value();
            json.get("geo_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    
    let missing: Vec<String> = geo_hashes
        .iter()
        .filter(|h| !found.contains(h))
        .cloned()
        .collect();
    
    Ok((found, missing))
}

/// 按参考号分组几何体数据
///
/// 将几何体数据按参考号分组，方便后续为每个参考号创建 XKT Entity
///
/// # 参数
/// * `geometries` - 几何体数据列表
///
/// # 返回
/// 返回 HashMap<RefnoEnum, Vec<XktGeometryData>>
///
/// # 示例
/// ```no_run
/// use aios_core::{query_xkt_geometries, group_geometries_by_refno};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let refnos = vec!["17496/266203".parse()?];
///     let geometries = query_xkt_geometries(&refnos).await?;
///     let groups = group_geometries_by_refno(geometries);
///     
///     for (refno, geos) in groups {
///         println!("{} 有 {} 个几何体", refno, geos.len());
///     }
///     Ok(())
/// }
/// ```
pub fn group_geometries_by_refno(
    geometries: Vec<XktGeometryData>,
) -> HashMap<RefnoEnum, Vec<XktGeometryData>> {
    let mut groups = HashMap::new();
    
    for geo in geometries {
        groups
            .entry(geo.refno.clone())
            .or_insert_with(Vec::new)
            .push(geo);
    }
    
    groups
}

/// 统计几何体信息
///
/// # 参数
/// * `geometries` - 几何体数据列表
///
/// # 返回
/// 返回 (总数, 唯一几何体数, 按类型统计)
pub fn get_geometry_statistics(
    geometries: &[XktGeometryData],
) -> (usize, usize, HashMap<String, usize>) {
    let total = geometries.len();
    
    // 统计唯一几何体
    let unique_hashes: std::collections::HashSet<_> = geometries
        .iter()
        .map(|g| &g.geo_hash)
        .collect();
    let unique_count = unique_hashes.len();
    
    // 按类型统计
    let mut by_type: HashMap<String, usize> = HashMap::new();
    for geo in geometries {
        *by_type.entry(geo.noun.clone()).or_insert(0) += 1;
    }
    
    (total, unique_count, by_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_geometries_by_refno() {
        let geometries = vec![
            XktGeometryData {
                refno: "17496/266203".parse().unwrap(),
                geo_hash: "hash1".to_string(),
                world_trans: Transform::default(),
                meshed: true,
                noun: "EQUI".to_string(),
                local_trans: None,
            },
            XktGeometryData {
                refno: "17496/266203".parse().unwrap(),
                geo_hash: "hash2".to_string(),
                world_trans: Transform::default(),
                meshed: true,
                noun: "EQUI".to_string(),
                local_trans: None,
            },
            XktGeometryData {
                refno: "24383/86525".parse().unwrap(),
                geo_hash: "hash3".to_string(),
                world_trans: Transform::default(),
                meshed: true,
                noun: "PIPE".to_string(),
                local_trans: None,
            },
        ];

        let groups = group_geometries_by_refno(geometries);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get(&"17496/266203".parse().unwrap()).unwrap().len(), 2);
        assert_eq!(groups.get(&"24383/86525".parse().unwrap()).unwrap().len(), 1);
    }

    #[test]
    fn test_get_geometry_statistics() {
        let geometries = vec![
            XktGeometryData {
                refno: "17496/266203".parse().unwrap(),
                geo_hash: "hash1".to_string(),
                world_trans: Transform::default(),
                meshed: true,
                noun: "EQUI".to_string(),
                local_trans: None,
            },
            XktGeometryData {
                refno: "17496/266203".parse().unwrap(),
                geo_hash: "hash1".to_string(), // 重复的 hash
                world_trans: Transform::default(),
                meshed: true,
                noun: "EQUI".to_string(),
                local_trans: None,
            },
            XktGeometryData {
                refno: "24383/86525".parse().unwrap(),
                geo_hash: "hash2".to_string(),
                world_trans: Transform::default(),
                meshed: true,
                noun: "PIPE".to_string(),
                local_trans: None,
            },
        ];

        let (total, unique, by_type) = get_geometry_statistics(&geometries);
        assert_eq!(total, 3);
        assert_eq!(unique, 2); // hash1 和 hash2
        assert_eq!(*by_type.get("EQUI").unwrap(), 2);
        assert_eq!(*by_type.get("PIPE").unwrap(), 1);
    }
}

