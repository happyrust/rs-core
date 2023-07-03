use std::collections::HashMap;
use bevy_ecs::prelude::Resource;
use glam::Vec3;
use serde::{Serialize, Deserialize, Serializer};
use serde::de::DeserializeOwned;
use uuid::Uuid;
use crate::data_center::AttrValue::{AttrFloat, AttrStrArray, AttrString};
use crate::metadata_manager::FileBytes;
use crate::pdms_types::RefU64;

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
    #[serde(rename = "projectCode")]
    pub project_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    pub version: String,
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
    pub value: String,
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
    AttrVec3(Vec3),
    AttrVec3Array(Vec<Vec3>),
    AttrMap(HashMap<String, Vec<String>>),
    AttrMapFloatArray(HashMap<String, Vec<f32>>),
    AttrItemArray(Vec<ItemValue>),
}

impl Default for AttrValue {
    fn default() -> Self {
        AttrString("".to_string())
    }
}

impl Into<String> for AttrValue {
    fn into(self) -> String {
        match self {
            AttrString(a) => {
                a
            }
            AttrFloat(a) => {
                a.to_string()
            }
            AttrValue::AttrInt(a) => {
                a.to_string()
            }
            AttrValue::AttrBool(a) => {
                if a { "Y".to_string() } else { "N".to_string() }
            }
            AttrStrArray(a) => {
                serde_json::to_string(&a).unwrap_or("[]".to_string())
            }
            AttrValue::AttrIntArray(a) => {
                serde_json::to_string(&a).unwrap_or("[]".to_string())
            }
            AttrValue::AttrFloatArray(a) => {
                serde_json::to_string(&a).unwrap_or("[]".to_string())
            }
            AttrValue::AttrVec3(a) => {
                serde_json::to_string(&a).unwrap_or("[]".to_string())
            }
            AttrValue::AttrVec3Array(a) => {
                serde_json::to_string(&a).unwrap_or("[]".to_string())
            }
            AttrValue::AttrMap(a) => {
                serde_json::to_string(&a).unwrap_or("{}".to_string())
            }
            AttrValue::AttrMapFloatArray(a) => {
                serde_json::to_string(&a).unwrap_or("{}".to_string())
            }
            AttrValue::AttrItemArray(a) => {
                serde_json::to_string(&a).unwrap_or("[]".to_string())
            }
        }
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ThreeDDatacenterRequest {
    pub title: String,
    pub refnos: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ThreeDDatacenterResponse {
    #[serde(rename = "Success")]
    pub success: bool,
    #[serde(rename = "Result")]
    pub result: String,
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "LoginUrl")]
    pub login_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TubiData {
    pub pre_refno: RefU64,
    pub lstu_name: String,
    pub length: f32,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SendHoleDataToArango {
    pub _key: String,
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "formdata")]
    pub form_data: SendHoleDataFormData,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SendHoleData {
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "formdata")]
    pub form_data: SendHoleDataFormData,
}

impl SendHoleData {
    pub fn to_arango_struct(self) -> SendHoleDataToArango {
        SendHoleDataToArango {
            _key: self.key_value.clone(),
            key_value:self.key_value,
            form_data: self.form_data,
        }
    }
}

impl SendHoleDataToArango {
    pub fn to_ui_struct(self) -> SendHoleData {
        SendHoleData {
            key_value: self.key_value,
            form_data: self.form_data,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SendHoleDataFormData {
    #[serde(rename = "ProjCode")]
    pub project_code: String,
    #[serde(rename = "HumanCode")]
    pub human_code: String,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Major")]
    pub major: String,
    #[serde(rename = "WXType")]
    pub wx_type: String,
    #[serde(rename = "JD_Name")]
    pub jd_name: String,
    #[serde(rename = "SH_Name")]
    pub sh_name: String,
    #[serde(rename = "SD_Name")]
    pub sd_name: String,
    #[serde(rename = "SZ_Name")]
    pub sz_name: String,
    #[serde(rename = "Memo")]
    pub memo: String,
    #[serde(rename = "databody")]
    pub data_body: DataCenterProject,
    #[serde(rename = "modelbody")]
    pub model_body: Vec<HoleDataModelBody>,
    #[serde(rename = "Detail")]
    pub detail: Vec<DataCenterDetail>,
    #[serde(rename = "files")]
    pub files: Vec<DataCenterFile>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HoleDataModelBody {
    pub code: String,
    pub status: bool,
    #[serde(rename = "JD")]
    pub jd: Vec<String>,
    #[serde(rename = "SH")]
    pub sh: Vec<String>,
    #[serde(rename = "SD")]
    pub sd: Vec<String>,
    #[serde(rename = "SZ")]
    pub sz: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterFile {
    #[serde(rename = "filename")]
    pub file_name: String,
    #[serde(rename = "filestream")]
    pub file_stream: String,
}

impl DataCenterFile {
    pub fn from_file_bytes(file_bytes: FileBytes) -> Self {
        Self {
            file_name: file_bytes.file_name,
            file_stream: base64::encode(&file_bytes.data),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterDetail {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Type")]
    pub detail_type: String,
    #[serde(rename = "Major")]
    pub major: String,
    #[serde(rename = "ActExplain")]
    pub act_explain: String,
    #[serde(rename = "Posi")]
    pub position: String,
    #[serde(rename = "Memo")]
    pub memo: String,
    #[serde(rename = "Upddate")]
    pub update: String,
    #[serde(rename = "ActHum")]
    pub act_hum: String,
    pub is_hole:bool,
    pub key:String
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
    let r = vec![item_1, item_2];
    let data = serde_json::to_string(&r).unwrap();
    dbg!(&data);
}