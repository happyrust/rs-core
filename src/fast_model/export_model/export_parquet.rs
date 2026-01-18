// 仅在启用相关 feature 时编译此模块，避免由于 polars 依赖缺失导致的编译错误
#[cfg(feature = "parquet")]
use crate::PlantTransform;
#[cfg(feature = "parquet")]
use crate::types::PlantAabb;
#[cfg(feature = "parquet")]
use crate::{RefnoEnum, SUL_DB, SurrealQueryExt};
#[cfg(feature = "parquet")]
use polars::df;
#[cfg(feature = "parquet")]
use polars::prelude::*;
use std::fs;
use std::path::Path;

#[cfg(feature = "parquet")]
/// 导出 inst_relate_aabb + world_trans 到 Parquet，用于空间计算
pub async fn export_inst_aabb_parquet(output_path: &Path) -> anyhow::Result<()> {
    // 查询字段：refno、dbnum、noun、world_trans（可选）、aabb（flatten）
    let sql = r#"
SELECT
  in.id as refno,
  in.noun as noun,
  in.dbnum as dbnum,
  world_trans.d as world_trans,
  in->inst_relate_aabb.out[0]
FROM inst_relate
WHERE world_trans.d != none
  AND array::len(in->inst_relate_aabb) > 0
"#;

    let rows: Vec<Row> = SUL_DB.query_take(sql, 0).await?;
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
    let mut skipped = 0usize;

    for row in rows {
        let Some(aabb) = row.aabb else {
            skipped += 1;
            continue;
        };
        let mins = aabb.mins();
        let maxs = aabb.maxs();
        refnos.push(row.refno);
        dbnos.push(row.dbnum as i64);
        nouns.push(row.noun.unwrap_or_default());
        min_x.push(mins.x as f64);
        max_x.push(maxs.x as f64);
        min_y.push(mins.y as f64);
        max_y.push(maxs.y as f64);
        min_z.push(mins.z as f64);
        max_z.push(maxs.z as f64);
    }

    if refnos.is_empty() {
        println!("[parquet] inst_relate_aabb 没有可用 aabb，跳过导出");
        return Ok(());
    }
    if skipped > 0 {
        println!("[parquet] inst_relate_aabb 过滤掉 {} 条空 aabb", skipped);
    }

    let df = df![
        "refno" => refnos,
        "dbnum" => dbnos,
        "noun" => nouns,
        "min_x" => min_x,
        "max_x" => max_x,
        "min_y" => min_y,
        "max_y" => max_y,
        "min_z" => min_z,
        "max_z" => max_z,
    ]?;

    let file_path = out_dir.join(
        output_path
            .file_name()
            .unwrap_or_else(|| "inst_aabb.parquet".as_ref()),
    );
    let file = std::fs::File::create(&file_path)?;
    ParquetWriter::new(file).finish(&df)?;

    println!(
        "[parquet] 导出 inst_relate_aabb -> {} (rows={})",
        file_path.display(),
        df.height()
    );
    Ok(())
}

#[cfg(feature = "parquet")]
#[derive(Debug, Clone, serde::Deserialize)]
struct Row {
    refno: String,
    dbnum: u32,
    noun: Option<String>,
    world_trans: Option<crate::PlantTransform>,
    #[serde(default)]
    aabb: Option<PlantAabb>,
}
