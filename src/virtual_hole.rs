use bevy::ecs::system::Resource;
use serde::Deserialize;
use serde::Serialize;
use crate::data_center::DataCenterProject;

#[derive(Resource, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct PersonnelInfo {
    #[serde(rename = "人员工号")]
    pub job_num: String,
    #[serde(rename = "人员名称")]
    pub name: String,
}


#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct PersonnelInfoVec {
    pub data: Vec<PersonnelInfo>,
}


// ///发送提资数据到普华
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct SendAuditDataToArango {
//     #[serde(rename = "KeyValue")]
//     pub _key: String,
//     #[serde(rename = "formdata")]
//     pub form_data: FormData,
//
// }
//
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct SendAuditData {
//     #[serde(rename = "KeyValue")]
//     pub key_value: String,
//     #[serde(rename = "formdata")]
//     pub form_data: FormData,
//
// }
//
// impl SendAuditData {
//     pub fn to_arango_struct(self) -> SendAuditDataToArango {
//         SendAuditDataToArango {
//             _key: self.key_value,
//             form_data: self.form_data,
//         }
//     }
// }
//
//
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct FormData {
//     #[serde(rename = "Title")]
//     pub title: String,
//     #[serde(rename = "ProjCode")]
//     pub proj_code: String,
//     #[serde(rename = "HumanCode")]
//     pub human_code: String,
//     #[serde(rename = "Major")]
//     pub major: String,
//     #[serde(rename = "WXType")]
//     pub wx_type: String,
//     #[serde(rename = "JD_Name")]
//     pub jd_name: String,
//     #[serde(rename = "SH_Name")]
//     pub sh_name: String,
//     #[serde(rename = "SD_Name")]
//     pub sd_name: String,
//     #[serde(rename = "SZ_Name")]
//     pub sz_name: String,
//     #[serde(rename = "Memo")]
//     pub memo: String,
//     #[serde(rename = "databody")]
//     pub data_body: DataCenterProject,
//     #[serde(rename = "modelbody")]
//     pub model_body: ModelBody,
//     #[serde(rename = "Detail")]
//     pub detail: Vec<AuditDetail>,
//     pub files: Vec<AuditFile>,
// }
//
//
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct ModelBody {
//     pub things: Vec<ModelThing>,
// }
//
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct ModelThing {
//     pub code: String,
//     pub JD: Vec<String>,
//     pub SH: Vec<String>,
//     pub SD: Vec<String>,
//     pub SZ: Vec<String>,
// }
//
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct AuditDetail {
//     #[serde(rename = "Code")]
//     pub code: String,
//     #[serde(rename = "Type")]
//     pub type_: String,
//     #[serde(rename = "Major")]
//     pub major: String,
//     #[serde(rename = "ActExplain")]
//     pub act_explain: String,
//     #[serde(rename = "Posi")]
//     pub posi: String,
//     #[serde(rename = "Memo")]
//     pub memo: String,
//     #[serde(rename = "Upddate")]
//     pub upd_date: String,
//     #[serde(rename = "ActHum")]
//     pub act_hum: String,
// }
//
// #[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
// pub struct AuditFile {
//     #[serde(rename = "filename")]
//     pub file_name: String,
//     #[serde(rename = "filestream")]
//     pub file_stream: String,
//
// }