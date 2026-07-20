//! specs/022：按 sesno 查询 PE/ATT 历史快照（SurrealDB RocksDB versioned）。
//!
//! 业务入口是 sesno；内部经 `sesno_version_anchor` 换算 `anchored_at`，再发
//! `SELECT … VERSION d'…'`。GC 水位线越界翻译为 [`HistoryError::Expired`]。

use crate::{RefnoEnum, SUL_DB, SurrealQueryExt};
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::types::SurrealValue;
use thiserror::Error;

/// 锚点命中结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorHit {
    pub dbnum: u32,
    pub sesno: u32,
    /// RFC3339 / Surreal datetime 字符串，可直接拼进 `VERSION d'…'`。
    pub anchored_at: String,
    pub source: Option<String>,
    /// `true` = 精确命中请求 sesno；`false` = 回退到「最近不大于」。
    pub exact: bool,
}

/// 单元素在某一锚点时刻的快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSnapshot {
    pub refno_u64: u64,
    pub pe_key: String,
    pub requested_sesno: u32,
    pub resolved_sesno: u32,
    pub exact_anchor: bool,
    pub anchored_at: String,
    /// 该时刻记录是否存在（硬删除后 VERSION 仍可能返回删除前态）。
    pub exists: bool,
    pub pe: Option<Value>,
    pub att: Option<Value>,
    pub noun: Option<String>,
}

/// 区间 diff 分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Unchanged,
    Changed,
    Added,
    Removed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub path: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementDiff {
    pub refno_u64: u64,
    pub pe_key: String,
    pub kind: DiffKind,
    pub from_sesno: u32,
    pub to_sesno: u32,
    pub changes: Vec<FieldChange>,
    pub from_snapshot: Option<ElementSnapshot>,
    pub to_snapshot: Option<ElementSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    pub sesno: u32,
    pub exact_anchor: bool,
    pub anchored_at: String,
    pub content_hash: String,
    pub exists: bool,
    pub changed_from_prev: bool,
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error(
        "该 sesno 历史已超出 retention 窗口，请改用源 db 文件重扫或放宽 version_retention（anchor={anchor}）"
    )]
    Expired { anchor: String, detail: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl HistoryError {
    fn map_query_err(err: anyhow::Error, anchor: &str) -> Self {
        let msg = err.to_string();
        if is_history_expired_message(&msg) {
            Self::Expired {
                anchor: anchor.to_string(),
                detail: msg,
            }
        } else {
            Self::Other(err)
        }
    }
}

/// 将任意 VERSION 查询错误统一映射为历史错误语义。
pub fn map_history_query_error(error: anyhow::Error, anchor: &str) -> HistoryError {
    HistoryError::map_query_err(error, anchor)
}

fn is_history_expired_message(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("invalidargument")
        || lower.contains("invalid argument")
        || lower.contains("below the garbage collection")
        || lower.contains("full_history_ts_low")
        || lower.contains("retention")
            && (lower.contains("version") || lower.contains("history") || lower.contains("gc"))
}

#[derive(Debug, Deserialize, SurrealValue)]
struct AnchorRow {
    dbnum: i64,
    sesno: i64,
    #[serde(default)]
    source: Option<String>,
    /// Surreal datetime；SDK 通常反序列化为可 Display 的值，这里用 String 承接。
    anchored_at: String,
}

fn datetime_literal(raw: &str) -> Result<String> {
    let s = normalize_datetime_literal(raw);
    if s.is_empty() {
        bail!("anchored_at 为空");
    }
    Ok(s)
}

fn normalize_datetime_literal(raw: &str) -> String {
    let s = raw.trim();
    let s = s
        .strip_prefix("d'")
        .and_then(|x| x.strip_suffix('\''))
        .unwrap_or(s);
    s.to_string()
}

fn row_to_hit(row: AnchorRow, exact: bool) -> Result<AnchorHit> {
    Ok(AnchorHit {
        dbnum: row.dbnum as u32,
        sesno: row.sesno as u32,
        anchored_at: datetime_literal(&row.anchored_at)?,
        source: row.source,
        exact,
    })
}

async fn resolve_anchor_for_source(
    dbnum: u32,
    sesno: u32,
    source_filter: &str,
) -> Result<Option<AnchorHit>> {
    let exact_sql = format!(
        "SELECT dbnum, sesno, source, type::string(anchored_at) AS anchored_at \
         FROM sesno_version_anchor \
         WHERE dbnum = {dbnum} AND sesno = {sesno} AND {source_filter} \
         ORDER BY anchored_at DESC LIMIT 1;"
    );
    let exact_rows: Vec<AnchorRow> = SUL_DB.query_take(&exact_sql, 0).await?;
    if let Some(row) = exact_rows.into_iter().next() {
        return Ok(Some(row_to_hit(row, true)?));
    }

    let fallback_sql = format!(
        "SELECT dbnum, sesno, source, type::string(anchored_at) AS anchored_at \
         FROM sesno_version_anchor \
         WHERE dbnum = {dbnum} AND sesno <= {sesno} AND {source_filter} \
         ORDER BY sesno DESC, anchored_at DESC LIMIT 1;"
    );
    let rows: Vec<AnchorRow> = SUL_DB.query_take(&fallback_sql, 0).await?;
    if let Some(row) = rows.into_iter().next() {
        return Ok(Some(row_to_hit(row, false)?));
    }
    Ok(None)
}

/// 解析 PE/ATT 数据锚点：只允许 `full`/`incremental`。
pub async fn resolve_data_anchor(dbnum: u32, sesno: u32) -> Result<Option<AnchorHit>> {
    resolve_anchor_for_source(dbnum, sesno, "source IN ['full', 'incremental']").await
}

/// 解析模型锚点：只允许 `model_gen`。
pub async fn resolve_model_anchor(dbnum: u32, sesno: u32) -> Result<Option<AnchorHit>> {
    resolve_anchor_for_source(dbnum, sesno, "source = 'model_gen'").await
}

async fn list_anchors_for_source(
    dbnum: u32,
    limit: usize,
    source_filter: &str,
) -> Result<Vec<AnchorHit>> {
    let limit_clause = if limit == 0 {
        String::new()
    } else {
        format!(" LIMIT {limit}")
    };
    let sql = format!(
        "SELECT dbnum, sesno, source, type::string(anchored_at) AS anchored_at \
         FROM sesno_version_anchor \
         WHERE dbnum = {dbnum} AND {source_filter} \
         ORDER BY sesno ASC, anchored_at ASC{limit_clause};"
    );
    let rows: Vec<AnchorRow> = SUL_DB.query_take(&sql, 0).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row_to_hit(row, true)?);
    }
    Ok(out)
}

/// 列出数据锚点。`limit` 为 0 表示不截断。
pub async fn list_data_anchors(dbnum: u32, limit: usize) -> Result<Vec<AnchorHit>> {
    list_anchors_for_source(dbnum, limit, "source IN ['full', 'incremental']").await
}

/// 列出模型锚点。`limit` 为 0 表示不截断。
pub async fn list_model_anchors(dbnum: u32, limit: usize) -> Result<Vec<AnchorHit>> {
    list_anchors_for_source(dbnum, limit, "source = 'model_gen'").await
}

/// 一个 refno 在同一 `model_gen` 锚点下的完整模型记录集。
///
/// `relations` 包含 refno 点记录及以 refno 为前缀的数组 ID 关系记录；
/// `inst_info` / `inst_geo` 是这些关系直接引用的记录，并且使用同一个
/// `anchored_at` 读取，避免把当前态引用混入历史快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSnapshot {
    pub refno_u64: u64,
    pub requested_sesno: u32,
    pub anchor: AnchorHit,
    pub exists: bool,
    pub relations: BTreeMap<String, Vec<Value>>,
    pub inst_info: BTreeMap<String, Value>,
    pub inst_geo: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecordChange {
    pub key: String,
    pub kind: DiffKind,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDiff {
    pub refno_u64: u64,
    pub from_requested_sesno: u32,
    pub to_requested_sesno: u32,
    pub from_anchor: AnchorHit,
    pub to_anchor: AnchorHit,
    pub kind: DiffKind,
    pub changes: Vec<ModelRecordChange>,
    pub from_snapshot: ModelSnapshot,
    pub to_snapshot: ModelSnapshot,
}

const MODEL_POINT_TABLES: &[&str] = &[
    "inst_relate",
    "inst_relate_bool",
    "inst_relate_cata_bool",
    "inst_relate_aabb",
    "inst_relate_booled_aabb",
    "refno_relations",
];

const MODEL_RANGE_TABLES: &[&str] = &["geo_relate", "neg_relate", "ngmr_relate", "tubi_relate"];

fn model_refno_parts(refno: RefnoEnum) -> (u32, u32) {
    let raw = refno.refno();
    (raw.get_0(), raw.get_1())
}

fn model_point_key(table: &str, ref0: u32, ref1: u32) -> String {
    format!("{table}:[{ref0},{ref1}]")
}

fn model_range_key(table: &str, ref0: u32, ref1: u32) -> String {
    format!("{table}:[{ref0},{ref1},NONE]..=[{ref0},{ref1},..]")
}

async fn model_tables_at(anchored_at: &str) -> Result<BTreeSet<String>, HistoryError> {
    let sql = format!("RETURN object::keys((INFO FOR DB VERSION d'{anchored_at}').tables);");
    let tables: Vec<String> = SUL_DB
        .query_take(&sql, 0)
        .await
        .map_err(|error| HistoryError::map_query_err(error, anchored_at))?;
    Ok(tables.into_iter().collect())
}

async fn select_model_rows_at(source: &str, anchored_at: &str) -> Result<Vec<Value>, HistoryError> {
    let sql = format!("SELECT * FROM {source} VERSION d'{anchored_at}';");
    SUL_DB
        .query_take::<Vec<Value>>(&sql, 0)
        .await
        .map_err(|error| HistoryError::map_query_err(error, anchored_at))
}

async fn select_record_ids_at(
    field: &str,
    source: &str,
    anchored_at: &str,
) -> Result<Vec<String>, HistoryError> {
    let sql = format!("SELECT VALUE type::string({field}) FROM {source} VERSION d'{anchored_at}';");
    SUL_DB
        .query_take::<Vec<String>>(&sql, 0)
        .await
        .map_err(|error| HistoryError::map_query_err(error, anchored_at))
}

fn retain_model_reference(ids: &mut BTreeSet<String>, raw: String, table: &str) {
    let raw = raw.trim();
    if raw.starts_with(&format!("{table}:"))
        && raw.len() <= 512
        && !raw.contains(';')
        && !raw.contains('\n')
        && !raw.contains('\r')
    {
        ids.insert(raw.to_string());
    }
}

async fn select_referenced_records_at(
    ids: &BTreeSet<String>,
    anchored_at: &str,
) -> Result<BTreeMap<String, Value>, HistoryError> {
    let mut records = BTreeMap::new();
    for id in ids {
        if let Some(value) = select_model_rows_at(id, anchored_at)
            .await?
            .into_iter()
            .next()
        {
            records.insert(id.clone(), value);
        }
    }
    Ok(records)
}

async fn model_snapshot_at_anchor(
    refno: RefnoEnum,
    requested_sesno: u32,
    anchor: AnchorHit,
) -> Result<ModelSnapshot, HistoryError> {
    let (ref0, ref1) = model_refno_parts(refno);
    let tables = model_tables_at(&anchor.anchored_at).await?;
    let mut relations = BTreeMap::new();

    for table in MODEL_POINT_TABLES {
        let rows = if tables.contains(*table) {
            let source = model_point_key(table, ref0, ref1);
            select_model_rows_at(&source, &anchor.anchored_at).await?
        } else {
            Vec::new()
        };
        relations.insert((*table).to_string(), rows);
    }
    for table in MODEL_RANGE_TABLES {
        let rows = if tables.contains(*table) {
            let source = model_range_key(table, ref0, ref1);
            select_model_rows_at(&source, &anchor.anchored_at).await?
        } else {
            Vec::new()
        };
        relations.insert((*table).to_string(), rows);
    }

    let inst_relate = model_point_key("inst_relate", ref0, ref1);
    let cata_bool = model_point_key("inst_relate_cata_bool", ref0, ref1);
    let geo_range = model_range_key("geo_relate", ref0, ref1);
    let tubi_range = model_range_key("tubi_relate", ref0, ref1);
    let mut inst_info_ids = BTreeSet::new();
    let mut inst_geo_ids = BTreeSet::new();

    if tables.contains("inst_relate") {
        for raw in select_record_ids_at("out", &inst_relate, &anchor.anchored_at).await? {
            retain_model_reference(&mut inst_info_ids, raw, "inst_info");
        }
    }
    if tables.contains("geo_relate") {
        for raw in select_record_ids_at("in", &geo_range, &anchor.anchored_at).await? {
            retain_model_reference(&mut inst_info_ids, raw, "inst_info");
        }
        for raw in select_record_ids_at("out", &geo_range, &anchor.anchored_at).await? {
            retain_model_reference(&mut inst_geo_ids, raw, "inst_geo");
        }
    }
    if tables.contains("inst_relate_cata_bool") {
        for raw in select_record_ids_at("in", &cata_bool, &anchor.anchored_at).await? {
            retain_model_reference(&mut inst_info_ids, raw, "inst_info");
        }
        for raw in select_record_ids_at("out", &cata_bool, &anchor.anchored_at).await? {
            retain_model_reference(&mut inst_geo_ids, raw, "inst_geo");
        }
    }
    if tables.contains("tubi_relate") {
        for raw in select_record_ids_at("geo", &tubi_range, &anchor.anchored_at).await? {
            retain_model_reference(&mut inst_geo_ids, raw, "inst_geo");
        }
    }

    let inst_info = if tables.contains("inst_info") {
        select_referenced_records_at(&inst_info_ids, &anchor.anchored_at).await?
    } else {
        BTreeMap::new()
    };
    let inst_geo = if tables.contains("inst_geo") {
        select_referenced_records_at(&inst_geo_ids, &anchor.anchored_at).await?
    } else {
        BTreeMap::new()
    };
    let exists = relations.values().any(|rows| !rows.is_empty())
        || !inst_info.is_empty()
        || !inst_geo.is_empty();

    Ok(ModelSnapshot {
        refno_u64: refno_u64(refno),
        requested_sesno,
        anchor,
        exists,
        relations,
        inst_info,
        inst_geo,
    })
}

/// 在不大于 `sesno` 的最近 `model_gen` 锚点读取一个 refno 的模型快照。
pub async fn model_snapshot_at(
    refno: RefnoEnum,
    sesno: u32,
    dbnum: u32,
) -> Result<ModelSnapshot, HistoryError> {
    let anchor = resolve_model_anchor(dbnum, sesno)
        .await
        .map_err(HistoryError::Other)?
        .ok_or_else(|| {
            HistoryError::Other(anyhow!(
                "未找到 dbnum={dbnum} sesno<={sesno} 的 model_gen anchor"
            ))
        })?;
    model_snapshot_at_anchor(refno, sesno, anchor).await
}

fn snapshot_record_map(snapshot: &ModelSnapshot) -> BTreeMap<String, Value> {
    let mut records = BTreeMap::new();
    for (table, rows) in &snapshot.relations {
        for (index, row) in rows.iter().enumerate() {
            let id = row
                .get("id")
                .map(Value::to_string)
                .unwrap_or_else(|| format!("#{index}"));
            records.insert(format!("relation/{table}/{id}"), row.clone());
        }
    }
    for (id, value) in &snapshot.inst_info {
        records.insert(format!("inst_info/{id}"), value.clone());
    }
    for (id, value) in &snapshot.inst_geo {
        records.insert(format!("inst_geo/{id}"), value.clone());
    }
    records
}

fn diff_model_snapshots(from: ModelSnapshot, to: ModelSnapshot) -> ModelDiff {
    let from_records = snapshot_record_map(&from);
    let to_records = snapshot_record_map(&to);
    let keys = from_records
        .keys()
        .chain(to_records.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    for key in keys {
        let old = from_records.get(&key);
        let new = to_records.get(&key);
        let kind = match (old, new) {
            (None, Some(_)) => DiffKind::Added,
            (Some(_), None) => DiffKind::Deleted,
            (Some(old), Some(new)) if old != new => DiffKind::Changed,
            _ => DiffKind::Unchanged,
        };
        if kind != DiffKind::Unchanged {
            changes.push(ModelRecordChange {
                key,
                kind,
                old: old.cloned(),
                new: new.cloned(),
            });
        }
    }
    let kind = match (from.exists, to.exists, changes.is_empty()) {
        (false, true, _) => DiffKind::Added,
        (true, false, _) => DiffKind::Deleted,
        (_, _, false) => DiffKind::Changed,
        _ => DiffKind::Unchanged,
    };
    ModelDiff {
        refno_u64: from.refno_u64,
        from_requested_sesno: from.requested_sesno,
        to_requested_sesno: to.requested_sesno,
        from_anchor: from.anchor.clone(),
        to_anchor: to.anchor.clone(),
        kind,
        changes,
        from_snapshot: from,
        to_snapshot: to,
    }
}

/// 批量比较两个 `model_gen` 锚点。所有 refno 共享同一对已解析锚点。
pub async fn model_diff(
    refnos: &[RefnoEnum],
    from_sesno: u32,
    to_sesno: u32,
    dbnum: u32,
) -> Result<Vec<ModelDiff>, HistoryError> {
    let from_anchor = resolve_model_anchor(dbnum, from_sesno)
        .await
        .map_err(HistoryError::Other)?
        .ok_or_else(|| {
            HistoryError::Other(anyhow!(
                "未找到 dbnum={dbnum} sesno<={from_sesno} 的 model_gen anchor"
            ))
        })?;
    let to_anchor = resolve_model_anchor(dbnum, to_sesno)
        .await
        .map_err(HistoryError::Other)?
        .ok_or_else(|| {
            HistoryError::Other(anyhow!(
                "未找到 dbnum={dbnum} sesno<={to_sesno} 的 model_gen anchor"
            ))
        })?;
    let mut diffs = Vec::with_capacity(refnos.len());
    for &refno in refnos {
        let from = model_snapshot_at_anchor(refno, from_sesno, from_anchor.clone()).await?;
        let to = model_snapshot_at_anchor(refno, to_sesno, to_anchor.clone()).await?;
        diffs.push(diff_model_snapshots(from, to));
    }
    Ok(diffs)
}

fn pe_key_for(refno: RefnoEnum) -> String {
    refno.to_pe_key()
}

fn refno_u64(refno: RefnoEnum) -> u64 {
    match refno {
        RefnoEnum::Refno(r) => r.0,
        RefnoEnum::SesRef(s) => s.refno.0,
    }
}

fn extract_noun(pe: &Value) -> Option<String> {
    pe.get("noun")
        .or_else(|| pe.get("TYPE"))
        .or_else(|| pe.get("type"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

async fn select_record_at(
    table_key: &str,
    anchored_at: &str,
) -> Result<Option<Value>, HistoryError> {
    let sql = format!("SELECT * FROM {table_key} VERSION d'{anchored_at}';");
    let rows: Vec<Value> = SUL_DB
        .query_take::<Vec<Value>>(&sql, 0)
        .await
        .map_err(|e| HistoryError::map_query_err(e, anchored_at))?;
    Ok(rows.into_iter().next())
}

/// 按 sesno 取单元素 PE(+ATT) 快照。`pe_key_override` 用于测试夹具（如 `pe:equi_001`）。
pub async fn snapshot_at(
    refno: RefnoEnum,
    sesno: u32,
    dbnum: Option<u32>,
    pe_key_override: Option<&str>,
) -> Result<ElementSnapshot, HistoryError> {
    let dbnum = match dbnum {
        Some(d) => d,
        None => {
            // 无 dbnum 时无法定位锚点表；要求调用方传入
            return Err(HistoryError::Other(anyhow!(
                "snapshot_at 需要 dbnum（锚点按 dbnum+sesno 键控）"
            )));
        }
    };

    let anchor = resolve_data_anchor(dbnum, sesno)
        .await
        .map_err(HistoryError::Other)?
        .ok_or_else(|| {
            HistoryError::Other(anyhow!(
                "未找到 dbnum={dbnum} sesno<={sesno} 的 sesno_version_anchor"
            ))
        })?;

    let pe_key = pe_key_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| pe_key_for(refno));
    let pe = select_record_at(&pe_key, &anchor.anchored_at).await?;
    let noun = pe.as_ref().and_then(extract_noun);
    let att = if let Some(n) = noun.as_deref() {
        // 真实落库：ATT 表名为 noun；测试夹具可能把 attrs 嵌在 PE 上
        let att_key = format!(
            "{}:{}",
            n,
            pe_key.split_once(':').map(|(_, id)| id).unwrap_or("")
        );
        // 仅当 pe_key 是标准 pe:<id> 时尝试；夹具 pe:equi_001 的 ATT 表可能不存在
        match select_record_at(&att_key, &anchor.anchored_at).await {
            Ok(v) => v,
            Err(HistoryError::Expired { .. }) => {
                return Err(HistoryError::Expired {
                    anchor: anchor.anchored_at.clone(),
                    detail: "att VERSION".into(),
                });
            }
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(ElementSnapshot {
        refno_u64: refno_u64(refno),
        pe_key,
        requested_sesno: sesno,
        resolved_sesno: anchor.sesno,
        exact_anchor: anchor.exact,
        anchored_at: anchor.anchored_at,
        exists: pe.is_some(),
        pe,
        att,
        noun,
    })
}

fn content_hash(snap: &ElementSnapshot) -> String {
    let payload = serde_json::json!({
        "exists": snap.exists,
        "pe": snap.pe,
        "att": snap.att,
    });
    // 轻量稳定摘要：不引入额外 crate，用 Debug 长度 + 关键字段
    format!("{:x}", fnv1a64(payload.to_string().as_bytes()))
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in data {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn flatten_changes(
    prefix: &str,
    old: Option<&Value>,
    new: Option<&Value>,
    out: &mut Vec<FieldChange>,
) {
    match (old, new) {
        (None, None) => {}
        (None, Some(n)) => out.push(FieldChange {
            path: prefix.to_string(),
            old: None,
            new: Some(n.clone()),
        }),
        (Some(o), None) => out.push(FieldChange {
            path: prefix.to_string(),
            old: Some(o.clone()),
            new: None,
        }),
        (Some(Value::Object(om)), Some(Value::Object(nm))) => {
            let mut keys: Vec<&String> = om.keys().chain(nm.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                if k == "id" {
                    continue;
                }
                let p = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_changes(&p, om.get(k), nm.get(k), out);
            }
        }
        (Some(o), Some(n)) if o == n => {}
        (Some(o), Some(n)) => out.push(FieldChange {
            path: prefix.to_string(),
            old: Some(o.clone()),
            new: Some(n.clone()),
        }),
    }
}

/// 两端快照字段级对比。`pe_key_override` 仅在单元素夹具场景使用（长度须为 1 或与 refnos 等长时取首个）。
pub async fn diff_range(
    refnos: &[RefnoEnum],
    from_sesno: u32,
    to_sesno: u32,
    dbnum: u32,
) -> Result<Vec<ElementDiff>, HistoryError> {
    diff_range_with_pe_keys(refnos, from_sesno, to_sesno, dbnum, None).await
}

pub async fn diff_range_with_pe_keys(
    refnos: &[RefnoEnum],
    from_sesno: u32,
    to_sesno: u32,
    dbnum: u32,
    pe_key_override: Option<&str>,
) -> Result<Vec<ElementDiff>, HistoryError> {
    let mut out = Vec::with_capacity(refnos.len());
    for &refno in refnos {
        let from = snapshot_at(refno, from_sesno, Some(dbnum), pe_key_override).await?;
        let to = snapshot_at(refno, to_sesno, Some(dbnum), pe_key_override).await?;
        let kind = match (from.exists, to.exists) {
            (false, false) => DiffKind::Unchanged,
            (false, true) => DiffKind::Added,
            (true, false) => DiffKind::Deleted,
            (true, true) => {
                let mut changes = Vec::new();
                flatten_changes("", from.pe.as_ref(), to.pe.as_ref(), &mut changes);
                flatten_changes("att", from.att.as_ref(), to.att.as_ref(), &mut changes);
                if changes.is_empty() {
                    DiffKind::Unchanged
                } else {
                    DiffKind::Changed
                }
            }
        };
        let mut changes = Vec::new();
        if matches!(
            kind,
            DiffKind::Changed | DiffKind::Added | DiffKind::Deleted
        ) {
            flatten_changes("", from.pe.as_ref(), to.pe.as_ref(), &mut changes);
            flatten_changes("att", from.att.as_ref(), to.att.as_ref(), &mut changes);
        }
        out.push(ElementDiff {
            refno_u64: refno_u64(refno),
            pe_key: pe_key_override
                .map(|s| s.to_string())
                .unwrap_or_else(|| pe_key_for(refno)),
            kind,
            from_sesno,
            to_sesno,
            changes,
            from_snapshot: Some(from),
            to_snapshot: Some(to),
        });
    }
    Ok(out)
}

/// 列出锚点区间内该元素内容发生变化的 sesno。
pub async fn timeline(
    refno: RefnoEnum,
    from_sesno: u32,
    to_sesno: u32,
    dbnum: u32,
) -> Result<Vec<TimelinePoint>, HistoryError> {
    timeline_with_pe_key(refno, from_sesno, to_sesno, dbnum, None).await
}

pub async fn timeline_with_pe_key(
    refno: RefnoEnum,
    from_sesno: u32,
    to_sesno: u32,
    dbnum: u32,
    pe_key_override: Option<&str>,
) -> Result<Vec<TimelinePoint>, HistoryError> {
    if to_sesno < from_sesno {
        return Err(HistoryError::Other(anyhow!(
            "to_sesno ({to_sesno}) < from_sesno ({from_sesno})"
        )));
    }
    let sql = format!(
        "SELECT dbnum, sesno, source, type::string(anchored_at) AS anchored_at \
         FROM sesno_version_anchor \
         WHERE dbnum = {dbnum} AND sesno >= {from_sesno} AND sesno <= {to_sesno} \
         AND source IN ['full', 'incremental'] \
         ORDER BY sesno ASC, anchored_at ASC;"
    );
    let rows: Vec<AnchorRow> = SUL_DB
        .query_take(&sql, 0)
        .await
        .map_err(HistoryError::Other)?;

    let mut points = Vec::new();
    let mut prev_hash: Option<String> = None;
    for row in rows {
        let at = datetime_literal(&row.anchored_at).map_err(HistoryError::Other)?;
        let snap = snapshot_at(refno, row.sesno as u32, Some(dbnum), pe_key_override).await?;
        let hash = content_hash(&snap);
        let changed = prev_hash.as_ref().map(|p| p != &hash).unwrap_or(true);
        points.push(TimelinePoint {
            sesno: row.sesno as u32,
            exact_anchor: true,
            anchored_at: at,
            content_hash: hash.clone(),
            exists: snap.exists,
            changed_from_prev: changed,
        });
        prev_hash = Some(hash);
    }
    Ok(points)
}

/// 供 CLI 把 `HistoryError` 打成用户可读字符串（含 Expired 固定文案）。
pub fn format_history_error(err: &HistoryError) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_datetime_strips_surreal_prefix() {
        assert_eq!(
            normalize_datetime_literal("d'2026-07-16T15:40:29.221547Z'"),
            "2026-07-16T15:40:29.221547Z"
        );
    }

    #[test]
    fn expired_message_detection() {
        assert!(is_history_expired_message(
            "InvalidArgument: timestamp below the garbage collection watermark"
        ));
    }
}
