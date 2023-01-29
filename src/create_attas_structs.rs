use glam::Vec3;
use crate::pdms_types::RefU64;
use serde_derive::{Deserialize, Serialize};

//显示需创建ATTA的refno及name
#[derive(Default, Clone, Debug,Serialize, Deserialize)]
pub struct ATTAPos {
    pub pos: Vec<Vec3>,
}

#[derive(Default, Clone, Debug,Serialize, Deserialize)]
pub struct ATTAPosVec {
    pub data: Vec<ATTAPos>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualHoleGraphNode {
    pub _key: String,
    pub intelld: i32,
    pub code: String,
    pub relyitem: String,
    pub mainitem: String,
    pub speciality: String,
    pub position: String,
    pub holework: String,
    pub workby: String,
    pub time: String,
    pub shape: String,
    pub ori: String,
    pub itemref: String,
    pub mainitemref: String,
    pub openitem: String,
    pub plugtype: String,
    pub sizeheigh: f32,
    pub sizewidth: f32,
    pub bankwidth: f32,
    pub bankheight: f32,
    pub hotdis: String,
    pub heatthick: f32,
    pub refno: String,
    pub fittrefno: String,
    pub subsmeterial: String,
    pub substhickness: f32,
    pub icreate: i32,
    pub substype: String,
    pub extentlength1: f32,
    pub extentlength2: f32,
    pub second: i32,
    pub rehole: i32,
    pub note: String,
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualEmbedGraphNode {
    pub _key: String,
    pub intelld: i32,
    pub code: String,
    pub relyitem: String,
    pub relyitemref: String,
    pub mainitem: String,
    pub speciality: String,
    pub position: String,
    pub ori: String,
    pub work: String,
    pub workby: String,
    pub time: String,
    pub standertype: String,
    pub openitem: String,
    pub holework: String,
    pub sizelength: f32,
    pub sizewidth: f32,
    pub sizethickness: f32,
    pub minthickness: f32,
    pub load: f32,
    pub mindistance: f32,
    pub subsmeterial: String,
    pub fittid: String,
    pub _ref: String,
    pub shape: String,
    pub note: String,
}