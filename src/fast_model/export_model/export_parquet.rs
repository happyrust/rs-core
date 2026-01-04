use aios_core::types::PlantAabb;
use aios_core::PlantTransform;
use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt};
use polars::prelude::*;
use std::fs;
use std::path::Path;
use surrealdb::types::SurrealValue;

/// 导出 inst_relate_aabb + world_trans 到 Parquet，用于空间计算
pub async fn export_inst_aabb_parquet(output_path: &Path) -> anyhow::Result<()> {
    // 查询字段：refno、dbno、noun、world_trans（可选）、aabb（flatten）
    let sql = r#"
SELECT
  in.id as refno,
  in.noun as noun,
  in.dbno as dbno,
  world_trans.d as world_trans,
  (SELECT aabb.d FROM inst_relate_aabb WHERE refno = in LIMIT 1)[0] as aabb
FROM inst_relate
WHERE world_trans.d != none
  AND (SELECT refno FROM inst_relate_aabb WHERE refno = in LIMIT 1) != NONE
"#;

    let rows: Vec<Row> = SUL_DB.query_take(sql, 0).await.unwrap_or_default();
    if rows.is_empty() {
        println!("[parquet] inst_relate_aabb 查询为空，跳过导出");
        return Ok(());
    }

    let out_dir = output_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new("assets/parquet").to_path_buf());
    fs::create_dir_all(&out_dir)?;

    // 组装列
    let mut refnos = Vec::with_capacity(rows.len());
    let mut dbnos = Vec::with_capacity(rows.len());
    let mut nouns = Vec::with_capacity(rows.len());
    let mut min_x = Vec::with_capacity(rows.len());
    let mut max_x = Vec::with_capacity(rows.len());
    let mut min_y = Vec::with_capacity(rows.len());
    let mut max_y = Vec::with_capacity(rows.len());
    let mut min_z = Vec::with_capacity(rows.len());
    let mut max_z = Vec::with_capacity(rows.len());

    for row in rows {
        refnos.push(row.refno);
        dbnos.push(row.dbno as i64);
        nouns.push(row.noun.unwrap_or_default());
        min_x.push(row.aabb.mins.x as f64);
        max_x.push(row.aabb.maxs.x as f64);
        min_y.push(row.aabb.mins.y as f64);
        max_y.push(row.aabb.maxs.y as f64);
        min_z.push(row.aabb.mins.z as f64);
        max_z.push(row.aabb.maxs.z as f64);
    }

    let df = df![
        "refno" => refnos,
        "dbno" => dbnos,
        "noun" => nouns,
        "min_x" => min_x,
        "max_x" => max_x,
        "min_y" => min_y,
        "max_y" => max_y,
        "min_z" => min_z,
        "max_z" => max_z,
    ]?;

    let file_path = out_dir.join(output_path.file_name().unwrap_or_else(|| "inst_aabb.parquet".as_ref()));
    let file = std::fs::File::create(&file_path)?;
    ParquetWriter::new(file).finish(&df)?;

    println!(
        "[parquet] 导出 inst_relate_aabb -> {} (rows={})",
        file_path.display(),
        df.height()
    );
    Ok(())
}

#[derive(Debug)]
#[derive(Debug, Clone, serde::Deserialize, SurrealValue)]
struct Row {
    refno: String,
    dbno: u32,
    noun: Option<String>,
    world_trans: Option<aios_core::PlantTransform>,
    aabb: PlantAabb,
}
