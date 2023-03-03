use std::collections::HashMap;
use serde::{Serialize, Deserialize, Serializer};
use serde::de::DeserializeOwned;
use crate::data_center::AttrValue::{AttrFloat, AttrStrArray, AttrString};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProject {
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProjectWithRelations {
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
    pub relations: Vec<DataCenterRelations>,
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

#[derive(Serialize,Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AttrValue {
    AttrString(String),
    AttrFloat(f32),
    AttrInt(i32),
    AttrBool(bool),
    AttrStrArray(Vec<String>),
    AttrIntArray(Vec<i32>),
    AttrMap(HashMap<String,Vec<String>>),
}

impl Default for AttrValue {
    fn default() -> Self {
        AttrString("".to_string())
    }
}

#[test]
fn test_attr_json() {
    let data = AttrStrArray(vec!["hello".to_string(),"world".to_string()]);
    let data = AttrFloat(1.0);
    let json = serde_json::to_string(&data).unwrap();
    dbg!(&json);
}
