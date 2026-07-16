//! specs/022：按 sesno 查询 PE/ATT 历史快照（SurrealDB RocksDB versioned）。
//!
//! 业务入口是 sesno；内部经 `sesno_version_anchor` 换算 `anchored_at`，再发
//! `SELECT … VERSION d'…'`。GC 水位线越界翻译为 [`HistoryError::Expired`]。

use crate::{RefnoEnum, SUL_DB, SurrealQueryExt};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
        "该 sesno 历史已超出 retention 窗口，请改用 DuckLake 存档或源文件重扫（anchor={anchor}）"
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

/// 解析锚点：精确命中，否则回退到同 dbnum 下 `sesno <= 请求` 的最大一条。
pub async fn resolve_anchor(dbnum: u32, sesno: u32) -> Result<Option<AnchorHit>> {
    // 精确
    // anchored_at 用 type::string 转出，避免 Datetime 无法直接落到 Rust String。
    let exact_sql = format!(
        "SELECT dbnum, sesno, source, type::string(anchored_at) AS anchored_at \
         FROM ONLY sesno_version_anchor:[{dbnum}, {sesno}];"
    );
    match SUL_DB.query_take::<Option<AnchorRow>>(&exact_sql, 0).await {
        Ok(Some(row)) => return Ok(Some(row_to_hit(row, true)?)),
        Ok(None) => {}
        Err(e) => {
            // ONLY 在记录不存在时可能报错，降级到回退查询
            log::debug!("resolve_anchor exact miss/err: {e}");
        }
    }

    // 最近不大于
    let fallback_sql = format!(
        "SELECT dbnum, sesno, source, type::string(anchored_at) AS anchored_at \
         FROM sesno_version_anchor \
         WHERE dbnum = {dbnum} AND sesno <= {sesno} \
         ORDER BY sesno DESC LIMIT 1;"
    );
    let rows: Vec<AnchorRow> = SUL_DB.query_take(&fallback_sql, 0).await?;
    if let Some(row) = rows.into_iter().next() {
        return Ok(Some(row_to_hit(row, false)?));
    }
    Ok(None)
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

async fn select_record_at(table_key: &str, anchored_at: &str) -> Result<Option<Value>, HistoryError> {
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

    let anchor = resolve_anchor(dbnum, sesno)
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

fn flatten_changes(prefix: &str, old: Option<&Value>, new: Option<&Value>, out: &mut Vec<FieldChange>) {
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
        if matches!(kind, DiffKind::Changed | DiffKind::Added | DiffKind::Deleted) {
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
         ORDER BY sesno ASC;"
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
