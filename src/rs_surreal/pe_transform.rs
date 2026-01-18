use anyhow::Result;
use bevy_transform::prelude::Transform;
use serde::Deserialize;
use std::collections::HashMap;
use surrealdb::types::SurrealValue;

use crate::{RefnoEnum, SUL_DB, SurrealQueryExt, gen_bytes_hash};
use crate::rs_surreal::PlantTransform;

#[derive(Debug, Clone, Default)]
pub struct PeTransformCache {
    pub local: Option<Transform>,
    pub world: Option<Transform>,
}

#[derive(Debug, Clone)]
pub struct PeTransformEntry {
    pub refno: RefnoEnum,
    pub local: Option<Transform>,
    pub world: Option<Transform>,
}

#[derive(Debug, Deserialize, SurrealValue)]
struct PeTransformRow {
    local_trans: Option<PlantTransform>,
    world_trans: Option<PlantTransform>,
}

const INSERT_CHUNK_SIZE: usize = 200;

pub async fn ensure_pe_transform_schema() -> Result<()> {
    let sql = r#"
DEFINE TABLE IF NOT EXISTS pe_transform SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS local_trans ON TABLE pe_transform TYPE option<record<trans>>;
DEFINE FIELD IF NOT EXISTS world_trans ON TABLE pe_transform TYPE option<record<trans>>;
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE pe_transform TYPE datetime DEFAULT time::now();
"#;
    SUL_DB.query(sql).await?;
    Ok(())
}

pub async fn query_pe_transform(refno: RefnoEnum) -> Result<Option<PeTransformCache>> {
    let sql = format!(
        "SELECT local_trans.d as local_trans, world_trans.d as world_trans FROM {} LIMIT 1",
        refno.to_table_key("pe_transform")
    );
    let row: Option<PeTransformRow> = SUL_DB.query_take(&sql, 0).await?;
    Ok(row.map(|r| PeTransformCache {
        local: r.local_trans.map(|t| t.0),
        world: r.world_trans.map(|t| t.0),
    }))
}

/// 从 pe_transform 表查询 Transform
/// 
/// # Arguments
/// * `refno` - 参考号
/// * `is_local` - true 返回 local_trans，false 返回 world_trans
/// 
/// # Returns
/// * `Ok(Some(Transform))` - 查询成功
/// * `Ok(None)` - 未找到或字段为空
pub async fn query_transform(refno: RefnoEnum, is_local: bool) -> Result<Option<Transform>> {
    let cache = query_pe_transform(refno).await?;
    Ok(cache.and_then(|c| if is_local { c.local } else { c.world }))
}

pub async fn save_pe_transform(refno: RefnoEnum, local: Option<Transform>, world: Option<Transform>) -> Result<()> {
    let entry = PeTransformEntry { refno, local, world };
    save_pe_transform_entries(&[entry]).await
}

pub async fn save_pe_transform_entries(entries: &[PeTransformEntry]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    ensure_pe_transform_schema().await?;

    let mut trans_map: HashMap<u64, String> = HashMap::new();
    let mut update_sql = String::new();

    for entry in entries {
        let (local_ref, local_hash) = to_trans_ref(&entry.local, &mut trans_map)?;
        let (world_ref, world_hash) = to_trans_ref(&entry.world, &mut trans_map)?;

        if local_ref == "NONE" && world_ref == "NONE" {
            continue;
        }

        if local_hash.is_none() && world_hash.is_none() {
            continue;
        }

        update_sql.push_str(&format!(
            "UPSERT {} SET local_trans = {}, world_trans = {}, updated_at = time::now();",
            entry.refno.to_table_key("pe_transform"),
            local_ref,
            world_ref
        ));
    }

    if !trans_map.is_empty() {
        let keys: Vec<_> = trans_map.keys().copied().collect();
        for chunk in keys.chunks(INSERT_CHUNK_SIZE) {
            let mut sql = String::new();
            for hash in chunk {
                if let Some(json) = trans_map.get(hash) {
                    sql.push_str(&format!(
                        "INSERT IGNORE INTO trans {{'id':trans:⟨{}⟩, 'd':{}}};",
                        hash, json
                    ));
                }
            }
            if !sql.is_empty() {
                SUL_DB.query(&sql).await?;
            }
        }
    }

    if !update_sql.is_empty() {
        SUL_DB.query(&update_sql).await?;
    }

    Ok(())
}

pub async fn clear_pe_transform(refno: RefnoEnum) -> Result<()> {
    let sql = format!(
        "UPSERT {} SET local_trans = NONE, world_trans = NONE, updated_at = time::now();",
        refno.to_table_key("pe_transform")
    );
    SUL_DB.query(&sql).await?;
    Ok(())
}

fn to_trans_ref(
    trans: &Option<Transform>,
    trans_map: &mut HashMap<u64, String>,
) -> Result<(String, Option<u64>)> {
    let Some(transform) = trans else {
        return Ok(("NONE".to_string(), None));
    };

    if transform.translation.is_nan()
        || transform.rotation.is_nan()
        || transform.scale.is_nan()
    {
        return Ok(("NONE".to_string(), None));
    }

    let hash = gen_bytes_hash(transform);
    trans_map
        .entry(hash)
        .or_insert(serde_json::to_string(transform)?);

    Ok((format!("trans:⟨{}⟩", hash), Some(hash)))
}
