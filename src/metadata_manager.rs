use serde_derive::{Deserialize, Serialize};
use crate::pdms_types::{PdmsElement, PdmsNodeTrait, RefU64};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataManagerTreeNode {
    pub id: u64,
    pub owner: u64,
    pub user_code: String,
    pub chinese_name: String,
    pub english_name: String,
}

impl PdmsNodeTrait for MetadataManagerTreeNode {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        RefU64(self.id)
    }

    #[inline]
    fn get_name(&self) -> &str {
        &self.chinese_name
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        0
    }

    #[inline]
    fn get_type_name(&self) -> &str { "" }

    #[inline]
    fn get_children_count(&self) -> usize { 1 }
}

impl Into<sled::IVec> for MetadataManagerTreeNode {
    fn into(self) -> sled::IVec {
        bincode::serialize(&self).unwrap().into()
    }
}

impl Into<sled::IVec> for &MetadataManagerTreeNode {
    fn into(self) -> sled::IVec {
        bincode::serialize(self).unwrap().into()
    }
}

impl From<sled::IVec> for MetadataManagerTreeNode {
    fn from(d: sled::IVec) -> Self {
        bincode::deserialize(&d).unwrap()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataManagerTableData {
    pub id: u64,
    pub code: String,
    pub name: String,
    pub b_null: bool,
    pub data_type: u8,
    pub unit: u8,
    pub desc: String,
    pub scope: String,
}

impl MetadataManagerTableData {
    pub fn convert_str_to_data_type(input: &str) -> u8 {
        match input.trim().to_lowercase().as_str() {
            "string" => { 1 }
            "float" => { 2 }
            &_ => { 0 }
        }
    }

    pub fn get_data_type_type_from_u8(input: u8) -> String {
        match input {
            1 => "String".to_string(),
            2 => "float".to_string(),
            _ => "".to_string()
        }
    }

    pub fn convert_str_to_unit(input: &str) -> u8 {
        match input.trim().to_lowercase().as_str() {
            "m" => { 1 }
            "dm" => { 2 }
            "cm" => { 3 }
            "mm" => { 4 }
            _ => 0
        }
    }

    pub fn get_unit_from_u8(input: u8) -> String {
        match input {
            1 => { "m".to_string() }
            2 => { "dm".to_string() }
            3 => { "cm".to_string() }
            4 => { "mm".to_string() }
            _ => "".to_string()
        }
    }
}