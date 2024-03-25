use std::collections::BTreeMap;
use crate::parsed_data::CateAxisParam;
use crate::types::*;
use serde::{Serialize,Deserialize};
use serde_with::serde_as;
use crate::pdms_types::ser_refno_as_str;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointMap{
    pub refno:RefU64,
    pub att_type: String,
    #[serde(default)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
}