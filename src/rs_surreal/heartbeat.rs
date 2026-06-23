//! 后台心跳保活 SUL_DB / 自定义 [`Surreal<Any>`] 单连接。
//!
//! SurrealDB 的 [`Lazy<Surreal<Any>>`](once_cell::sync::Lazy) 单连接 idle 一段时间后会
//! 被远端关闭，但 SDK 不会自动重连，导致下一次 `query.await` 永久 hang。
//! 心跳每 `interval` 跑一次 `RETURN 1` 维持连接活跃；如果心跳本身失败，
//! 立即把 [`super::CONNECTION_MANAGER`] 推到 `Disconnected`，下一次业务 query
//! 入口会走 `try_revive` 兜底（参见 [`super::any_resilient`]）。
//!
//! 触发事故 / 完整方案见：
//! - `<plant-model-gen>/docs/plans/2026-05-07-rs-core-sul-db-idle-resilience-plan.md`
//! - `<plant-model-gen>/docs/plans/2026-05-07-rs-core-sul-db-rollout-plan.md`

use std::time::Duration;

use log::{trace, warn};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

use super::CONNECTION_MANAGER;

/// 在后台 spawn 一条心跳任务，定期对 `db` 执行 `RETURN 1`。
///
/// # 参数
///
/// - `db`：被保活的 [`Surreal<Any>`] 单例（'static 生命周期）。
/// - `interval`：心跳间隔。建议 ≥ 30s，默认推荐 45s。
/// - `query_timeout`：每次心跳 query 的等待上限。建议 5s 内。
///
/// # 行为
///
/// - 心跳成功：trace 日志，不动状态机。
/// - 心跳失败（query Err 或 timeout）：warn 日志 + `CONNECTION_MANAGER.mark_disconnected()`。
/// - 永远不在心跳失败时 panic / 退出 task：靠业务 query 入口的 revive 修复。
///
/// # 关停
///
/// 返回 [`tokio::task::JoinHandle`]，调用方可在进程关停时 `.abort()`。
/// 如果不主动 abort，task 与进程一同 exit。
pub fn spawn_heartbeat(
    db: &'static Surreal<Any>,
    interval: Duration,
    query_timeout: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            match tokio::time::timeout(query_timeout, db.query("RETURN 1")).await {
                Ok(Ok(_resp)) => {
                    trace!("[sul-db] heartbeat ok");
                }
                Ok(Err(e)) => {
                    warn!("[sul-db] heartbeat query err: {e}");
                    CONNECTION_MANAGER.mark_disconnected().await;
                }
                Err(_elapsed) => {
                    warn!(
                        "[sul-db] heartbeat timeout after {:?} (mark disconnected)",
                        query_timeout
                    );
                    CONNECTION_MANAGER.mark_disconnected().await;
                }
            }
        }
    })
}

/// 默认心跳间隔（45s）。比典型 SurrealDB / 中间网关的 idle 超时小，足以保活。
pub const HEARTBEAT_INTERVAL_DEFAULT: Duration = Duration::from_secs(45);

/// 默认心跳 query 超时（5s）。比业务 query 的 30s 短，避免心跳本身阻塞过久。
pub const HEARTBEAT_QUERY_TIMEOUT_DEFAULT: Duration = Duration::from_secs(5);
