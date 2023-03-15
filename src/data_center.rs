use std::collections::HashMap;
use serde::{Serialize, Deserialize, Serializer};
use serde::de::DeserializeOwned;
use uuid::Uuid;
use crate::data_center::AttrValue::{AttrFloat, AttrStrArray, AttrString};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProject {
    #[serde(rename = "packageCode")]
    pub package_code: String,
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
}

impl DataCenterProject {
    pub fn convert_package_code() -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProjectWithRelations {
    #[serde(rename = "packageCode")]
    pub package_code: String,
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
    pub relations: Vec<DataCenterRelations>,
}

impl DataCenterProjectWithRelations {
    pub fn convert_package_code() -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterInstance {
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    pub attributes: Vec<DataCenterAttr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterRelations {
    pub version: String,
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    #[serde(rename = "startObjectCode")]
    pub start_object_code: String,
    #[serde(rename = "startInstanceCode")]
    pub start_instance_code: String,
    #[serde(rename = "endObjectCode")]
    pub end_object_code: String,
    #[serde(rename = "endInstanceCode")]
    pub end_instance_code: String,
    pub attributes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterAttr {
    #[serde(rename = "attributeModelCode")]
    pub attribute_model_code: String,
    pub value: AttrValue,
    // pub value: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AttrValue {
    AttrString(String),
    AttrFloat(f32),
    AttrInt(i32),
    AttrBool(bool),
    AttrStrArray(Vec<String>),
    AttrIntArray(Vec<i32>),
    AttrFloatArray(Vec<f32>),
    AttrMap(HashMap<String, Vec<String>>),
    AttrMapFloatArray(HashMap<String, Vec<f32>>),
    AttrItemArray(Vec<ItemValue>),
}

impl Default for AttrValue {
    fn default() -> Self {
        AttrString("".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ItemValue {
    String(String),
    Int(i32),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HoleType {
    // 孔洞类
    STUCJ,
    // 钢制套管类
    STUCG,
    // 纤维水泥电缆导管类
    STUCH,
    // 槽类
    STUCK,
    // 地坑类
    STUCL,
    // 地漏类
    STUCM,
    Unknown,
}

#[test]
fn test_attr_json() {
    let data = AttrStrArray(vec!["hello".to_string(), "world".to_string()]);
    let data = AttrFloat(1.0);
    let json = serde_json::to_string(&data).unwrap();
    dbg!(&json);
}

#[test]
fn test_item_value() {
    let item_1 = ItemValue::String("hello".to_string());
    let item_2 = ItemValue::Int(1);
    let r = vec![item_1,item_2];
    let data = serde_json::to_string(&r).unwrap();
    dbg!(&data);
}