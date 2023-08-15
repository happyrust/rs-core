use std::collections::BTreeMap;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::RefU64;
use serde::{Serialize,Deserialize};
use serde_with::serde_as;
use crate::pdms_types::ser_refno_as_str;
use crate::pdms_types::de_refno_from_key_str;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointMap{
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub refno:RefU64,
    pub att_type: String,
    #[serde(default)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
}