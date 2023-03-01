use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug,Default)]
pub struct DataCenterProject{
    #[serde(rename = "projectCode")]
    pub project_code:String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
}

#[derive(Serialize, Deserialize, Clone, Debug,Default)]
pub struct DataCenterProjectWithRelations{
    #[serde(rename = "projectCode")]
    pub project_code:String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
    pub relations: Vec<DataCenterRelations>,
}

#[derive(Serialize, Deserialize, Clone, Debug,Default)]
pub struct DataCenterInstance {
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    pub attributes: Vec<DataCenterAttr>,
}

#[derive(Serialize, Deserialize, Clone, Debug,Default)]
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

#[derive(Serialize, Deserialize, Clone, Debug,Default)]
pub struct DataCenterAttr {
    #[serde(rename = "attributeModelCode")]
    pub attribute_model_code: String,
    pub value: String,
}