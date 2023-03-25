use glam::Vec3;
use bevy::ecs::system::Resource;
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, Resource)]
pub struct VirtualHoleGraphNode {
    pub _key: String, // node identifier
    pub intelld: i32, // node type
    pub code: String, // node code
    pub relyitem: String, // node link
    pub mainitem: String, // node main item
    pub speciality: String, // node speciality
    pub position: String, // node position
    pub holework: String, // node work
    pub workby: String, // node work by
    pub time: String, // node time
    pub shape: String, // node shape
    pub ori: String, // node orientation
    pub itemref: String, // node item reference
    pub mainitemref: String, // node main item reference
    pub openitem: String, // node open item
    pub plugtype: String, // node plug type
    pub sizeheigh: f32, // node height
    pub sizewidth: f32, // node width
    pub bankwidth: f32, // node bank width
    pub bankheight: f32, // node bank height
    pub hotdis: String, // node hot distance
    pub heatthick: f32, // node heat thickness
    pub refno: String, // node reference number
    pub fittrefno: String, // node fitting reference number
    pub subsmeterial: String, // node subsurface material
    pub substhickness: f32, // node subsurface thickness
    pub icreate: i32, // node create
    pub substype: String, // node subsurface type
    pub extentlength1: f32, // node extent length 1
    pub extentlength2: f32, // node extent length 2
    pub second: i32, // node second
    pub rehole: i32, // node rehole
    pub note: String, // node note
}


// This function gets the key of a virtual embed graph node from the
// intelld, code, and relyitem fields. The key is used to store
// the node in the graph and to search for the node in the graph.

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualEmbedGraphNode {
    pub _key: String, // the name of the node
    pub intelld: i32, // the intelligence of the node
    pub code: String, // the code of the node
    pub relyitem: String, // the item that the node relies on
    pub relyitemref: String, // the reference of the item that the node relies on
    pub mainitem: String, // the main item
    pub speciality: String, // the specialty of the node
    pub position: String, // the position of the node
    pub ori: String, // the orientation of the node
    pub work: String, // the work of the node
    pub workby: String, // the worker of the node
    pub time: String, // the time of the node
    pub standertype: String, // the standard type of the node
    pub openitem: String, // the open item of the node
    pub holework: String, // the hole work of the node
    pub sizelength: f32, // the length of the node
    pub sizewidth: f32, // the width of the node
    pub sizethickness: f32, // the thickness of the node
    pub minthickness: f32, // the minimum thickness of the node
    pub load: f32, // the load of the node
    pub mindistance: f32, // the minimum distance of the node
    pub subsmeterial: String, // the subsurface meterial of the node
    pub fittid: String, // the fitting ID of the node
    pub _ref: String, // the reference of the node
    pub shape: String, // the shape of the node
    pub note: String, // the note of the node
}