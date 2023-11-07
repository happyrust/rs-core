use crate::RefU64;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// pub fn de_refno_from_sur_record<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
//     where
//         D: Deserializer<'de>,
// {
//     let s = String::deserialize(deserializer)?;
//     Ok(RefU64::from_url_refno(&s).unwrap_or_default())
// }
//
// // 需要和db num 关联在一起
// pub fn ser_refno_as_sur_record<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
// {
//     s.serialize_str(format!("{}:{}", refno.to_url_refno()))
// }
