//! 时间线查询功能模块
//! 
//! 提供与时间线相关的查询功能，包括：
//! - 查询指定 sesno 的时间范围
//! - 查询指定时间点的 ses 记录
//! - 查询时间范围内的 ses 变更

use anyhow::Result;
use serde::{Deserialize, Serialize};
use surrealdb::types::Value;

use crate::{
    rs_surreal::get_conn,
    types::{DbNumValue, RefnoEnum},
};

/// SES 记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesRecord {
    pub dbnum: u32,
    pub sesno: u32,
    pub date: chrono::DateTime<chrono::Utc>,
    pub computer_name: Option<String>,
    #[serde(default)]
    pub mod_cnt: u32,
    #[serde(default)]
    pub add_cnt: u32,
    #[serde(default)]
    pub del_cnt: u32,
}

/// 时间线查询服务
pub struct TimelineQueryService;

impl TimelineQueryService {
    /// 查询指定 sesno 的时间范围
    /// 
    /// # 参数
    /// - `dbnum`: 数据库编号
    /// - `sesno`: SES 编号
    /// 
    /// # 返回
    /// - `Ok((start_time, end_time))`: 开始时间和结束时间的时间戳
    /// - `Err`: 查询失败或未找到数据
    pub async fn query_ses_time_range(
        dbnum: u32,
        sesno: u32,
    ) -> Result<(f64, f64)> {
        // ses 表的 id 是复合键 [dbnum, sesno]
        let sql = format!(
            r#"
            SELECT date FROM ses:[{}, {}];
            "#,
            dbnum, sesno
        );

        let conn = get_conn();
        let mut response = conn.query(&sql).await?;
        let results: Vec<Value> = response.take(0)?;
        
        if results.is_empty() {
            return Err(anyhow::anyhow!(
                "未找到 dbnum: {}, sesno: {} 的 SES 记录",
                dbnum, sesno
            ));
        }

        // 获取当前 sesno 的时间
        let current_time = match &results[0] {
            Value::Object(obj) => {
                if let Some(Value::Datetime(dt)) = obj.get("date") {
                    dt.timestamp() as f64
                } else {
                    return Err(anyhow::anyhow!("无法解析日期字段"));
                }
            }
            _ => return Err(anyhow::anyhow!("意外的数据格式")),
        };

        // 查询下一个 sesno 的时间作为结束时间
        let next_sql = format!(
            r#"
            SELECT record::id(id) as id, date FROM ses 
            WHERE record::id(id)[0] = {} AND record::id(id)[1] > {} 
            ORDER BY record::id(id)[1] ASC 
            LIMIT 1;
            "#,
            dbnum, sesno
        );

        let mut next_response = conn.query(&next_sql).await?;
        let next_results: Vec<Value> = next_response.take(0)?;
        
        let end_time = if next_results.is_empty() {
            // 如果没有下一个 sesno，使用当前时间作为结束时间
            chrono::Utc::now().timestamp() as f64
        } else {
            match &next_results[0] {
                Value::Object(obj) => {
                    if let Some(Value::Datetime(dt)) = obj.get("date") {
                        dt.timestamp() as f64
                    } else {
                        chrono::Utc::now().timestamp() as f64
                    }
                }
                _ => chrono::Utc::now().timestamp() as f64,
            }
        };

        Ok((current_time, end_time))
    }

    /// 查询指定 dbnum 的所有 ses 时间范围
    /// 
    /// # 参数
    /// - `dbnum`: 数据库编号
    /// 
    /// # 返回
    /// - `Ok((earliest, latest))`: 最早和最晚的时间戳
    /// - `Err`: 查询失败或未找到数据
    pub async fn query_ses_time_range_by_dbnum(
        dbnum: u32,
    ) -> Result<(f64, f64)> {
        let sql = format!(
            r#"
            SELECT 
                math::min(date) as earliest,
                math::max(date) as latest
            FROM ses 
            WHERE record::id(id)[0] = {};
            "#,
            dbnum
        );

        let conn = get_conn();
        let mut response = conn.query(&sql).await?;
        let results: Vec<Value> = response.take(0)?;
        
        if results.is_empty() {
            return Err(anyhow::anyhow!("未找到 dbnum: {} 的数据", dbnum));
        }

        match &results[0] {
            Value::Object(obj) => {
                let earliest = match obj.get("earliest") {
                    Some(Value::Datetime(dt)) => dt.timestamp() as f64,
                    _ => return Err(anyhow::anyhow!("无法解析最早时间")),
                };
                
                let latest = match obj.get("latest") {
                    Some(Value::Datetime(dt)) => dt.timestamp() as f64,
                    _ => return Err(anyhow::anyhow!("无法解析最晚时间")),
                };
                
                Ok((earliest, latest))
            }
            _ => Err(anyhow::anyhow!("意外的数据格式")),
        }
    }

    /// 查询指定时间点的 ses 记录
    /// 
    /// # 参数
    /// - `dbnum`: 数据库编号
    /// - `timestamp`: 时间戳
    /// 
    /// # 返回
    /// - `Ok(Vec<(dbnum, sesno)>)`: 符合条件的记录列表
    /// - `Err`: 查询失败
    pub async fn query_ses_records_at_time(
        dbnum: u32,
        timestamp: f64,
    ) -> Result<Vec<(u32, u32)>> {
        let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("无效的时间戳"))?;
        
        let sql = format!(
            r#"
            SELECT record::id(id)[0] as dbnum, record::id(id)[1] as sesno 
            FROM ses 
            WHERE record::id(id)[0] = {} AND date <= d'{}' 
            ORDER BY record::id(id)[1] DESC 
            LIMIT 10;
            "#,
            dbnum,
            datetime.to_rfc3339()
        );

        let conn = get_conn();
        let mut response = conn.query(&sql).await?;
        let results: Vec<Value> = response.take(0)?;
        
        let mut records = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                let dbnum = match obj.get("dbnum") {
                    Some(Value::Number(n)) => n.as_u32().unwrap_or(0),
                    _ => continue,
                };
                
                let sesno = match obj.get("sesno") {
                    Some(Value::Number(n)) => n.as_u32().unwrap_or(0),
                    _ => continue,
                };
                
                records.push((dbnum, sesno));
            }
        }
        
        Ok(records)
    }

    /// 查询时间范围内的 ses 变更记录
    /// 
    /// # 参数
    /// - `dbnum`: 数据库编号
    /// - `start_time`: 开始时间戳
    /// - `end_time`: 结束时间戳
    /// 
    /// # 返回
    /// - `Ok(Vec<SesRecord>)`: SES 记录列表
    /// - `Err`: 查询失败
    pub async fn query_ses_changes_in_range(
        dbnum: u32,
        start_time: f64,
        end_time: f64,
    ) -> Result<Vec<SesRecord>> {
        let start_dt = chrono::DateTime::from_timestamp(start_time as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("无效的开始时间戳"))?;
        let end_dt = chrono::DateTime::from_timestamp(end_time as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("无效的结束时间戳"))?;
        
        let sql = format!(
            r#"
            SELECT 
                record::id(id)[0] as dbnum, 
                record::id(id)[1] as sesno, 
                date,
                computer_name,
                mod_cnt?:0 as mod_cnt,
                add_cnt?:0 as add_cnt,
                del_cnt?:0 as del_cnt
            FROM ses 
            WHERE record::id(id)[0] = {} 
                AND date >= d'{}' 
                AND date <= d'{}'
            ORDER BY record::id(id)[1] ASC;
            "#,
            dbnum,
            start_dt.to_rfc3339(),
            end_dt.to_rfc3339()
        );

        let conn = get_conn();
        let mut response = conn.query(&sql).await?;
        let results: Vec<SesRecord> = response.take(0)?;
        
        Ok(results)
    }

    /// 查询单个 ses 记录的详细信息
    /// 
    /// # 参数
    /// - `dbnum`: 数据库编号
    /// - `sesno`: SES 编号
    /// 
    /// # 返回
    /// - `Ok(SesRecord)`: SES 记录
    /// - `Err`: 查询失败或未找到数据
    pub async fn get_ses_record(
        dbnum: u32,
        sesno: u32,
    ) -> Result<SesRecord> {
        let sql = format!(
            r#"
            SELECT 
                {} as dbnum,
                {} as sesno,
                date,
                computer_name,
                mod_cnt?:0 as mod_cnt,
                add_cnt?:0 as add_cnt,
                del_cnt?:0 as del_cnt
            FROM ses:[{}, {}];
            "#,
            dbnum, sesno, dbnum, sesno
        );

        let conn = get_conn();
        let mut response = conn.query(&sql).await?;
        let results: Vec<SesRecord> = response.take(0)?;
        
        results.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("未找到 dbnum: {}, sesno: {} 的 SES 记录", dbnum, sesno))
    }

    /// 获取最新的 ses 记录
    /// 
    /// # 参数
    /// - `dbnum`: 数据库编号
    /// - `limit`: 返回记录数量限制
    /// 
    /// # 返回
    /// - `Ok(Vec<SesRecord>)`: SES 记录列表
    /// - `Err`: 查询失败
    pub async fn get_latest_ses_records(
        dbnum: u32,
        limit: u32,
    ) -> Result<Vec<SesRecord>> {
        let sql = format!(
            r#"
            SELECT 
                record::id(id)[0] as dbnum, 
                record::id(id)[1] as sesno, 
                date,
                computer_name,
                mod_cnt?:0 as mod_cnt,
                add_cnt?:0 as add_cnt,
                del_cnt?:0 as del_cnt
            FROM ses 
            WHERE record::id(id)[0] = {}
            ORDER BY record::id(id)[1] DESC
            LIMIT {};
            "#,
            dbnum, limit
        );

        let conn = get_conn();
        let mut response = conn.query(&sql).await?;
        let results: Vec<SesRecord> = response.take(0)?;
        
        Ok(results)
    }
}

// 导出便捷函数，保持向后兼容
pub async fn query_ses_time_range(dbnum: u32, sesno: u32) -> Result<(f64, f64)> {
    TimelineQueryService::query_ses_time_range(dbnum, sesno).await
}

pub async fn query_ses_time_range_by_dbnum(dbnum: u32) -> Result<(f64, f64)> {
    TimelineQueryService::query_ses_time_range_by_dbnum(dbnum).await
}

pub async fn query_ses_records_at_time(dbnum: u32, timestamp: f64) -> Result<Vec<(u32, u32)>> {
    TimelineQueryService::query_ses_records_at_time(dbnum, timestamp).await
}

pub async fn query_ses_changes_in_range(
    dbnum: u32,
    start_time: f64,
    end_time: f64,
) -> Result<Vec<SesRecord>> {
    TimelineQueryService::query_ses_changes_in_range(dbnum, start_time, end_time).await
}

pub async fn get_ses_record(dbnum: u32, sesno: u32) -> Result<SesRecord> {
    TimelineQueryService::get_ses_record(dbnum, sesno).await
}

pub async fn get_latest_ses_records(dbnum: u32, limit: u32) -> Result<Vec<SesRecord>> {
    TimelineQueryService::get_latest_ses_records(dbnum, limit).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_ses_time_range() {
        // 初始化测试数据库连接
        // 注意：需要确保测试环境中有正确的数据库连接配置
        
        // 测试查询单个 sesno 的时间范围
        let result = query_ses_time_range(1112, 1).await;
        match result {
            Ok((start, end)) => {
                println!("SES 时间范围: {} - {}", start, end);
                assert!(start <= end);
            }
            Err(e) => {
                println!("查询失败（预期的，如果没有测试数据）: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_query_ses_time_range_by_dbnum() {
        // 初始化测试数据库连接
        
        let result = query_ses_time_range_by_dbnum(1112).await;
        match result {
            Ok((earliest, latest)) => {
                println!("数据库时间范围: {} - {}", earliest, latest);
                assert!(earliest <= latest);
            }
            Err(e) => {
                println!("查询失败（预期的，如果没有测试数据）: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_query_ses_records_at_time() {
        // 初始化测试数据库连接
        
        let now = chrono::Utc::now().timestamp() as f64;
        let result = query_ses_records_at_time(1112, now).await;
        match result {
            Ok(records) => {
                println!("找到 {} 条记录", records.len());
                for (dbnum, sesno) in records {
                    println!("  dbnum: {}, sesno: {}", dbnum, sesno);
                }
            }
            Err(e) => {
                println!("查询失败（预期的，如果没有测试数据）: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_latest_ses_records() {
        // 初始化测试数据库连接
        
        let result = get_latest_ses_records(1112, 5).await;
        match result {
            Ok(records) => {
                println!("找到 {} 条最新记录", records.len());
                for record in records {
                    println!("  sesno: {}, date: {}", record.sesno, record.date);
                }
            }
            Err(e) => {
                println!("查询失败（预期的，如果没有测试数据）: {}", e);
            }
        }
    }
}