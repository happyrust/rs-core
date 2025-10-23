use surrealdb::types::{RecordId, RecordIdKey, ToSql};

/// 将表名与主键元组便捷地转换为 `RecordId`
pub trait IntoRecordId {
    fn into_record_id(self) -> RecordId;
}

impl IntoRecordId for (&str, &str) {
    fn into_record_id(self) -> RecordId {
        RecordId::new(self.0, self.1)
    }
}

impl IntoRecordId for (String, String) {
    fn into_record_id(self) -> RecordId {
        RecordId::new(self.0, self.1)
    }
}

impl IntoRecordId for (&str, String) {
    fn into_record_id(self) -> RecordId {
        RecordId::new(self.0, self.1)
    }
}

impl IntoRecordId for (String, &str) {
    fn into_record_id(self) -> RecordId {
        RecordId::new(self.0, self.1)
    }
}

/// `RecordId` 实用扩展
pub trait RecordIdExt {
    /// 返回形如 `table:key` 的原始字符串
    fn to_raw(&self) -> String;
    
    /// 提取用于 mesh 文件名的纯 ID 字符串
    /// 
    /// - 对于数字 ID (如 inst_geo:⟨123⟩)，返回 "123"
    /// - 对于字符串 ID (如 pe:17496_201377)，返回 "17496_201377"
    /// - 对于其他类型，返回完整的原始字符串
    fn to_mesh_id(&self) -> String;
}

impl RecordIdExt for RecordId {
    fn to_raw(&self) -> String {
        let mut raw = String::new();
        self.fmt_sql(&mut raw);
        raw
    }
    
    fn to_mesh_id(&self) -> String {
        match &self.key {
            RecordIdKey::Number(num) => num.to_string(),
            RecordIdKey::String(s) => s.clone(),
            _ => self.to_raw(),
        }
    }
}
