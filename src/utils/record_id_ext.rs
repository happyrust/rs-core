use surrealdb::types::{RecordId, RecordIdKey, ToSql};

/// 将表名与主键元组便捷地转换为 `RecordId`
pub trait IntoRecordId {
    fn into_record_id(self) -> RecordId;
}

impl<T, U> IntoRecordId for (T, U)
where
    T: Into<String>,
    U: Into<RecordIdKey>,
{
    fn into_record_id(self) -> RecordId {
        RecordId::new(self.0, self.1)
    }
}

/// `RecordId` 实用扩展
pub trait RecordIdExt {
    /// 返回形如 `table:key` 的原始字符串
    fn to_raw(&self) -> String;
}

impl RecordIdExt for RecordId {
    fn to_raw(&self) -> String {
        let mut raw = String::new();
        self.fmt_sql(&mut raw);
        raw
    }
}
