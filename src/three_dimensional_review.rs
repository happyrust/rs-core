use std::collections::HashMap;
use bevy_ecs::prelude::Resource;
use bevy_transform::prelude::Transform;
use serde::{Serialize, Deserialize};
use crate::pdms_types::RefU64;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource)]
pub struct ThreeDimensionalModelDataCrate {
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "ProjCode")]
    pub proj_code: String,
    #[serde(rename = "UserCode")]
    pub user_code: String,
    #[serde(rename = "SiteCode")]
    pub site_code: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "UserRole")]
    pub user_role: String,
    #[serde(rename = "ModelData")]
    pub model_data: ModelData,
    #[serde(rename = "FlowPicData")]
    pub flow_pic_data: ThreeDimensionalReviewComment,
}

impl ThreeDimensionalModelDataCrate {
    pub fn to_arango_struct(self) -> ThreeDimensionalModelDataToArango {
        ThreeDimensionalModelDataToArango {
            key_value: self.key_value,
            proj_code: self.proj_code,
            user_code: self.user_code,
            site_code: self.site_code,
            site_name: self.site_name,
            user_role: self.user_role,
            model_data: self.model_data,
            flow_pic_data: self.flow_pic_data,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalModelDataToArango {
    #[serde(rename = "_key")]
    pub key_value: String,
    #[serde(rename = "ProjCode")]
    pub proj_code: String,
    #[serde(rename = "UserCode")]
    pub user_code: String,
    #[serde(rename = "SiteCode")]
    pub site_code: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "UserRole")]
    pub user_role: String,
    #[serde(rename = "ModelData")]
    pub model_data: ModelData,
    #[serde(rename = "FlowPicData")]
    pub flow_pic_data: ThreeDimensionalReviewComment,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalReviewComment {
    pub validation: Vec<ThreeDimensionalReviewData>,
    pub review: Vec<ThreeDimensionalReviewData>,
    pub approval: Vec<ThreeDimensionalReviewData>,
    pub endorsement: Vec<ThreeDimensionalReviewData>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalReviewData {
    pub comment: Vec<String>,
    pub reply: Vec<String>,
    pub associatedElement: Vec<RefU64>,
    pub cloudLine: String,
    #[serde(rename = "viewpoint")]
    pub camera_transform: Transform,
    pub image: Vec<u8>,
    pub status: bool,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelDataIndex {
    pub refno: RefU64,
    pub name: String,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource)]
pub struct ModelData {
    pub index: Vec<ModelDataIndex>,
    pub data: Vec<HashMap<String, String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq,Hash)]
pub enum VagueSearchCondition {
    #[default]
    And,
    Or,
    Not,
}

impl Into<String> for VagueSearchCondition {
    fn into(self) -> String {
        match self {
            VagueSearchCondition::And => { "并且".to_string() }
            VagueSearchCondition::Or => { "或者".to_string() }
            VagueSearchCondition::Not => { "不含".to_string() }
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchRequest {
    // pub name: String,
    pub filter_refnos: Vec<RefU64>,
    // key : 过滤的类型 name , type 等  value: 0 : 过滤条件 and or not  1 : 过滤的值
    pub filter_condition: Vec<(String, (VagueSearchCondition, String))>,
}

/// 模糊查询导出为csv文件的数据
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchExportCsvData {
    pub key_word: String,
    pub result: String,
    pub belong_level: String,
    pub att_type: String,
}

impl VagueSearchExportCsvData {
    /// 将结构体数据转为Vec<String>，方便导出csv
    pub fn into_vec_string(self) -> Vec<String> {
        vec![self.key_word, self.result, self.belong_level, self.att_type]
    }
}

/// 模糊查询 从图数据库中查询到的需要导出的数据
#[serde_as]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchExportAqlData {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub name: String,
    pub level: Vec<String>,
    pub att_type: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchExportRequest {
    pub condition: String,
    pub refnos: Vec<RefU64>,
}