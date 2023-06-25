use bevy::ecs::system::Resource;
use serde::Deserialize;
use serde::Serialize;
use crate::data_center::DataCenterProject;
use crate::pdms_types::RefU64;

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

/// 封堵材料返回的数据
#[derive(PartialEq, Resource, Debug, Serialize, Deserialize)]
pub struct PluggingHoleData {
    // 孔洞的参考号
    pub hole_refno: RefU64,
    // 孔洞的 name
    pub hole_name: String,
    // 孔洞的尺寸( 圆形:直径 ，方形:长 宽 )
    pub hole_size: HoleSize,
    // 孔洞两边的房间
    pub hole_rooms: (String,String),
    // 电缆的占用面积
    pub cable_area: f32,
    // 防火封堵材料面积
    pub plugging_area: f32,
    // 封堵体积
    pub plugging_volume: f32,
    // 封堵材料
    pub plugging_material: String,
}

#[derive(PartialEq, Resource, Debug, Serialize, Deserialize)]
pub enum HoleSize {
    Circle(CircleHoleSize),
    Rect(RectHoleSize),
}

/// 圆形孔洞尺寸（不含高度）
#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct CircleHoleSize {
    pub radius: f32,
    pub height: f32,
}

/// 方形孔洞尺寸（不含高度）
#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct RectHoleSize {
    pub length: f32,
    pub width: f32,
    pub height: f32,
}

/// 孔洞封堵方式
#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct HoleBlockageMethod {
    pub method: String,
    pub thickness: f32,
}