use bevy_transform::prelude::Transform;
use glam::{Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::{EleInstGeo, RefU64};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmGeoInfo {
    pub _key: String,
    pub aabb: Option<Aabb>,
    pub data: Vec<EleInstGeo>,
    // 相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: Transform,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmGeoInfos {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub att_type: String,
    pub world_transform: Transform,
    pub rvm_inst_geo : Vec<RvmInstGeo>,
}

/// rvm 需要的 元件 geo 数据
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmInstGeo {
    pub geo_param: PdmsGeoParam,
    pub aabb: Option<Aabb>,
    //相对于自身的坐标系变换
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
}