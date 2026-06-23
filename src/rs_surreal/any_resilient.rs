//! Any-typed `Surreal<Any>` 弹性查询入口。
//!
//! 与 [`super::query_ext::SurrealQueryExt`] 的 generic 实现互补：
//! 这里的入口**专为 `SUL_DB`（`Surreal<Any>`）服务**，自动接入
//! [`super::CONNECTION_MANAGER`] 的 `mark_disconnected` / `try_revive`，
//! 让事故路径不仅有 30s timeout 兜底，还会在连接级失败后**自动重连一次**。
//!
//! 用法：把现有 `SUL_DB.query(sql).await` 或 `SUL_DB.query_response(sql).await`
//! 渐进迁到 [`query_response_resilient`]。
//!
//! 触发事故 / 完整方案见：
//! - `<plant-model-gen>/docs/plans/2026-05-07-rs-core-sul-db-idle-resilience-plan.md`
//! - `<plant-model-gen>/docs/plans/2026-05-07-rs-core-sul-db-rollout-plan.md`

use anyhow::Result;
use log::{debug, warn};
use surrealdb::IndexedResults as Response;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

use super::CONNECTION_MANAGER;
use super::query_ext::SurrealQueryExt;

/// 在 [`Surreal<Any>`] 上执行 query，附带 timeout（来自 [`super::query_ext`]）+ 自动重连。
///
/// # 流程
///
/// 1. 入口先 snapshot [`super::CONNECTION_MANAGER`] 状态；如果是 `Disconnected`，
///    用 `last_config` 调 `try_revive` 尝试一次（成功无副作用，失败仅记 warn）。
/// 2. 调 [`SurrealQueryExt::query_response`] 真执行 query（含 30s timeout）。
/// 3. 如果失败且 message 看起来是连接级（timeout / WS / IO / channel closed / connection），
///    把 [`super::CONNECTION_MANAGER`] 推到 `Disconnected`，下一次再走 step 1。
///
/// # 调用方约定
///
/// - 仅适用于 [`super::SUL_DB`]（[`Surreal<Any>`] 的全局单例）。  
///   其他 `Surreal<C>` 类型应继续用 generic [`SurrealQueryExt::query_response`]。
/// - 错误向上抛 anyhow，调用方应当返 5xx 给客户端。
/// - **不**做无限自动重试：每次调用最多一次 revive，避免雪崩。
pub async fn query_response_resilient(db: &Surreal<Any>, sql: impl AsRef<str>) -> Result<Response> {
    if CONNECTION_MANAGER.is_disconnected().await {
        match CONNECTION_MANAGER.try_revive(db).await {
            Ok(()) => debug!("[sul-db] revive 前置成功"),
            Err(e) => warn!("[sul-db] revive 前置失败（仍尝试 query，由 timeout 兜底）: {e}"),
        }
    }

    let result = db.query_response(sql).await;
    if let Err(ref err) = result {
        if is_connection_dead(&err.to_string()) {
            CONNECTION_MANAGER.mark_disconnected().await;
        }
    }
    result
}

/// 启发式判断 anyhow 错误信息是否提示连接级失败。
///
/// 这里覆盖：
/// - `query_ext` 的 timeout 文案
/// - SurrealDB SDK 的 IO / WebSocket / channel error
/// - 网络栈常见的 connection / disconnect 文案
fn is_connection_dead(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("query timeout")
        || lower.contains("send_error")
        || lower.contains("senderror")
        || lower.contains("io error")
        || lower.contains("websocket")
        || lower.contains("channel closed")
        || lower.contains("channel error")
        || lower.contains("connection")
        || lower.contains("disconnect")
        || lower.contains("broken pipe")
}

#[cfg(test)]
mod tests {
    use super::is_connection_dead;

    #[test]
    fn classifies_timeout_as_dead() {
        assert!(is_connection_dead("query timeout after 30s"));
    }

    #[test]
    fn classifies_websocket_as_dead() {
        assert!(is_connection_dead("WebSocket failed: connection reset"));
    }

    #[test]
    fn classifies_channel_closed_as_dead() {
        assert!(is_connection_dead(
            "Failed to send query results to channel: SendError(..)"
        ));
    }

    #[test]
    fn keeps_business_error_alive() {
        assert!(!is_connection_dead("namespace not found"));
        assert!(!is_connection_dead("permission denied: missing role"));
    }
}
