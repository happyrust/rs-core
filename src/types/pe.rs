use super::RefnoEnum;
use crate::RefU64;
use crate::pdms_types::{EleOperation, PdmsElement};
use crate::tool::db_tool::db1_hash;
use crate::types::named_attmap::NamedAttrMap;
use crate::types::named_attvalue::NamedAttrValue;
use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty};
use surrealdb::types::SurrealValue;
use surrealdb::types as surrealdb_types;
use serde_with::DisplayFromStr;
use std::fmt::format;

#[derive(Serialize, Deserialize, Clone, Debug, Resource, Default, PartialEq, SurrealValue)]
pub struct SPdmsElement {
    //指向具体的类型
    pub refno: RefnoEnum,
    pub owner: RefnoEnum,
    pub name: String,
    pub noun: String,
    pub dbnum: i32,
    ///小版本号
    pub sesno: i32,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<String>,
    pub cata_hash: String,
    ///锁定模型
    pub lock: bool,
    //todo 可以改为使用 op 来表达是否删除
    pub deleted: bool,
    #[serde(default)]
    pub op: EleOperation,

    /// TYPEX 扩展类型ID - 从 ATT_UDTYPE 或 ATT_TYPEX 属性中提取
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typex: Option<i32>,
}

impl SPdmsElement {
    #[inline]
    pub fn refno(&self) -> RefU64 {
        self.refno.refno()
    }

    #[inline]
    pub fn history_id(&self) -> String {
        format!("pe:{}_{}", self.refno(), self.sesno)
    }

    pub fn gen_sur_json(&self, id: Option<String>) -> String {
        let mut json_string = to_string_pretty(&json!({
            "name": self.name,
            "noun": self.noun,
            "dbnum": self.dbnum,
            "sesno": self.sesno,
            "status_code": self.status_code,
            "cata_hash": self.cata_hash,
            "lock": self.lock,
            "deleted": self.deleted,
        }))
        .unwrap();
        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(
            r#""refno": {},"#,
            self.refno().to_table_key(&self.noun)
        ));
        // 使用提供的 id，或默认使用 noun:refno 格式 (RecordId 类型)
        let id_value = id.unwrap_or_else(|| self.refno().to_table_key(&self.noun));
        json_string.push_str(&format!(r#""id": {},"#, id_value));
        json_string.push_str(&format!(r#""owner": {}"#, self.owner.to_pe_key()));
        json_string.push_str("}");
        json_string
    }

    //owner 的 sesno 也需要指定，不然不知道指向的是哪一个
    pub fn gen_sur_json_with_sesno(&self, sesno: i32, owner_sesno: i32) -> String {
        let mut json_string = to_string_pretty(&json!({
            "name": self.name,
            "noun": self.noun,
            "dbnum": self.dbnum,
            "sesno": self.sesno,
            "status_code": self.status_code,
            "cata_hash": self.cata_hash,
            "lock": self.lock,
            "deleted": self.deleted,
        }))
        .unwrap();
        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(
            r#""refno": {}_H:['{}',{}], "#,
            &self.noun,
            self.refno(),
            sesno
        ));
        json_string.push_str(&format!(r#""id": ['{}',{}],"#, self.refno(), sesno));
        if owner_sesno != 0 {
            json_string.push_str(&format!(
                r#""owner": pe:['{}',{}]"#,
                self.owner.refno(),
                owner_sesno
            ));
        } else {
            json_string.push_str(&format!(r#""owner": {}"#, self.owner.to_pe_key()));
        }
        json_string.push_str("}");
        json_string
    }

    #[inline]
    pub fn get_type_str(&self) -> &str {
        return self.noun.as_str();
    }

    #[inline]
    pub fn get_owner(&self) -> RefnoEnum {
        return self.owner;
    }

    /// 从 NamedAttrMap 中提取 TYPEX 值
    /// 优先级: UDTYPE > TYPEX > None
    pub fn extract_typex(&mut self, attmap: &NamedAttrMap) {
        // 1. 优先从 UDTYPE 属性获取
        if let Some(NamedAttrValue::IntegerType(type_id)) = attmap.map.get("UDTYPE") {
            self.typex = Some(*type_id);
            return;
        }

        // 2. 从 TYPEX 属性获取
        if let Some(NamedAttrValue::IntegerType(type_id)) = attmap.map.get("TYPEX") {
            self.typex = Some(*type_id);
            return;
        }

        // 3. 如果都没有，保持为 None
        self.typex = None;
    }

    /// 获取 noun 的 hash 值
    #[inline]
    pub fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }
}
