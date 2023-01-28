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
