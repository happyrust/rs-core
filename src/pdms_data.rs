use std::fmt::{Debug, Formatter};
use smol_str::SmolStr;
use serde::{Serialize,Deserialize};
use crate::pdms_types::{AttrMap, RefU64};


#[derive(Clone, Debug)]
pub struct ScomInfo {
    pub attr_map: AttrMap,
    pub gtype: SmolStr,
    pub dtse_params: Vec<DatasetParamStr>,
    pub gm_params: Vec<GmParam>,
    pub axis_params: Vec<AxisParam>,
    pub params: SmolStr,
    pub axis_param_numbers: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct DatasetParamStr {
    pub refno: SmolStr,
    pub name: SmolStr,
    pub self_type: SmolStr,
    pub lock: bool,
    pub owner: SmolStr,
    pub description: SmolStr,
    pub dkey: SmolStr,
    pub ptype: SmolStr,
    pub pproperty: SmolStr,
    pub dproperty: SmolStr,
    pub purpose: SmolStr,
    pub number: i32,
    pub dtitle: SmolStr,
    pub punits: SmolStr,
    pub ruse: SmolStr,
    pub lhide: bool,
}

//还是要用枚举，来列举各个情况
//GMSE GMSS
#[derive(Clone, Debug, Default)]
pub struct GmParam {
    pub refno: RefU64,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub gm_type: SmolStr,  //SCYL  LSNO  SCTO  SDSH  SBOX  SANN  SPRO

    pub prad: SmolStr,
    pub pang: SmolStr,
    pub pwid: SmolStr,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: Vec<SmolStr>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: Vec<SmolStr>,
    pub shears: Vec<SmolStr>,
    pub phei: SmolStr,
    pub offset: SmolStr,
    /// 顺序 x y z
    pub box_lengths: Vec<SmolStr>,
    pub xyz: Vec<SmolStr>,

    // pub profile:
    //profile  SPVE   SANN(PX, PY)
    pub verts: Vec<[SmolStr; 2]>,
    //SANN: dx dy dradius dwidth
    pub dxy: Vec<[SmolStr; 2]>,
    pub drad: SmolStr,
    pub dwid: SmolStr,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: Vec<SmolStr>,
    pub centre_line_flag: bool,
    pub visible_flag: bool,
    pub prads: Vec<SmolStr>,
}

#[derive(Clone, Debug, Default)]
pub struct AxisParam {
    pub attr_map: AttrMap,
    pub x: SmolStr,
    pub y: SmolStr,
    pub z: SmolStr,
    pub distance: SmolStr,
    pub direction: SmolStr,
    pub pconnect: SmolStr,
    pub pbore: SmolStr,
}

#[derive(Debug,Serialize,Deserialize)]
pub enum NewDataState {
    Modify,
    Increase,
    Delete
}

#[derive(Serialize,Deserialize)]
pub struct IncrementData {
    pub refno: RefU64,
    pub attr_data_map: AttrMap,
    pub state: NewDataState,
    pub version: u32,
}


impl Debug for IncrementData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementData")
            .field("refno",&self.refno.to_refno_str())
            .field("map",&self.attr_data_map.to_string_hashmap())
            .field("state",&self.state)
            .field("version",&self.version)
            .finish()
    }
}