use std::collections::BTreeMap;
use crate::parsed_data::CateAxisParam;
use crate::types::*;
use serde::{Serialize, Deserialize};
use serde_with::serde_as;
use crate::pdms_types::ser_refno_as_str;
use crate::pdms_types::de_refno_from_key_str;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointMap {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub refno: RefU64,
    pub att_type: String,
    #[serde(default)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointVec {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub att_type: String,
    #[serde(default)]
    pub ptset_map: Vec<CateAxisParam>,
}

impl InstPointVec {
    pub fn into_point_map(self) -> InstPointMap {
        let mut map = BTreeMap::new();
        for p in self.ptset_map {
            map.entry(p.number).or_insert(p);
        }
        InstPointMap {
            refno: self.id,
            att_type: self.att_type,
            ptset_map: map,
        }
    }
}