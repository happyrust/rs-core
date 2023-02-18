use bevy::prelude::World;
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
    pub data_type: String,
    pub unit: String,
    pub desc: String,
    pub scope: String,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileBytes {
    pub data: Vec<u8>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ShowMetadataManagerTableData {
    pub id: u64,
    pub old_code: String,
    pub new_code: String,
    pub name: String,
    pub b_null: String,
    pub data_type: String,
    pub unit: String,
    pub desc: String,
    pub scope: String,
    pub change: bool,
}


impl ShowMetadataManagerTableData {
    pub fn init(table_data: MetadataManagerTableData) -> ShowMetadataManagerTableData {
        let mut data = ShowMetadataManagerTableData::default();
        data.id = table_data.id;
        data.old_code = table_data.code.clone();
        data.new_code = table_data.code;
        data.name = table_data.name;
        if table_data.b_null {
            data.b_null = "是".to_string();
        } else {
            data.b_null = "否".to_string();
        }
        data.data_type = table_data.data_type;
        data.unit = table_data.unit;
        data.desc = table_data.desc;
        data.scope = table_data.scope;
        data.change = false;
        data
    }
}