use crate::pdms_types::EleOperation;
use crate::{RefU64, RefnoEnum, SUL_DB, SurrealQueryExt};
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use surrealdb::types as surrealdb_types;
use surrealdb::types::{Datetime, SurrealValue};
use surrealdb::IndexedResults as Response;

/// 版本信息，包含版本号、日期、作者、变更统计
#[derive(Clone, Debug, Serialize, Deserialize, SurrealValue)]
pub struct VersionInfo {
    /// db 号和 sesno 的组合
    pub id: String,
    pub date: Datetime,
    /// 可以延迟去获取
    #[serde(skip)]
    pub changes: Vec<ChangeDetail>,
    pub author: String,
    pub add_cnt: usize,
    pub mod_cnt: usize,
    pub del_cnt: usize,
}

impl VersionInfo {
    #[inline]
    pub fn total_cnt(&self) -> usize {
        self.add_cnt + self.mod_cnt + self.del_cnt
    }

    #[inline]
    pub fn local_dt(&self) -> NaiveDateTime {
        self.date.naive_local()
    }
}

/// 变更计数统计
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeCount {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
}

/// 变更类型枚举
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

/// 变更详情，记录单个元素的变更
#[derive(Clone, Debug, Serialize, Deserialize, SurrealValue)]
pub struct ChangeDetail {
    pub refno: RefnoEnum,
    pub name: String,
    pub op: EleOperation,
}

/// 版本项，用于版本历史记录
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionItem {
    pub version: String,
    pub date: NaiveDate,
    pub changes: Vec<ChangeDetail>,
}

/// PE（PipelineElement）的历史数据，记录单个元素的版本历史
#[derive(Clone, Debug, Serialize, Deserialize, SurrealValue)]
pub struct PEHistoryData {
    pub refno: RefnoEnum,
    pub name: String,
    pub version: u32,
    pub author: String,
    pub op: EleOperation,
    pub date: String,
}

/// 查询指定参考号的历史数据
pub async fn query_pe_history_data(refno: RefU64) -> Result<Vec<PEHistoryData>> {
    let sql = format!(
        r#"select id as refno, fn::default_full_name(id)?:'' as name, op?:0 as op, sesno as version,
                    fn::ses_data(id).date?:'' as date, fn::ses_data(id).computer_name?:'' as author
            from his_pe:{0}.refnos, pe:{0} order by sesno desc"#,
        refno.to_string()
    );
    let mut response: Response = SUL_DB.query_response(&sql).await?;
    let his_data: Vec<PEHistoryData> = response.take(0)?;
    Ok(his_data)
}
