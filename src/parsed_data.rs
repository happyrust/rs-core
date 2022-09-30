use std::collections::BTreeMap;
use bevy::utils::HashMap;
use dashmap::DashMap;
use glam::{Vec2, Vec3};
use serde_derive::{Deserialize, Serialize};
use smol_str::SmolStr;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::RefU64;

#[derive(Clone, PartialEq, Debug)]
pub struct DesignPipeRequest {
    // pub name: String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct DesignComponentRequest {
    
    pub name: String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct DesignBranRequest {
    
    pub name: String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct RefnosRequest {
    
    pub name: String,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Refnos {
    
    pub refnos: Vec<String>,
}
#[derive(Clone, Debug, Default)]
pub struct DesignPipe {
    pub name: String,
    pub refno: String,
    pub brans: Vec<DesignBran>,
}
#[derive(Clone, Debug, Default)]
pub struct DesignBran {
    pub name: String,
    pub refno: String,
    pub components: Vec<GeomsInfo>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeomsInfo {
    pub geometries: Vec<CateGeoParam>,
    pub axis_map: BTreeMap<i32, CateAxisParam>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Dataset {
    pub self_type: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GmseParamData {
    pub refno: RefU64,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub type_name: SmolStr,
    pub radius: f32, //desi 里的radius
    pub angle: f32, //desi 里的angle
    ///desi 里的height
    pub height: f32,
    pub pwid: f32,
    pub pang: f32,  //元件库里的angle
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: Vec<f32>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: Vec<f32>,
    pub shears: Vec<f32>,
    /// 元件库里的height
    pub phei: f32,
    pub offset: f32,
    /// 顶点集合
    pub verts: Vec<Vec3>,
    pub dxy: Vec<[f32; 2]>,
    pub drad: f32,
    pub dwid: f32,
    pub box_lengths: Vec<f32>,
    /// 顺序 x y z
    pub xyz: Vec<f32>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: Vec<CateAxisParam>,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    /// Fillet radius
    pub frads: Vec<f32>,
    pub prad: f32,

    /// plin 数据
    pub plin_verts: Vec2,
    pub plin_plax: Vec3,
}



#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct CateAxisParam {
    pub refno: RefU64,
    pub number: i32,
    pub pt: Vec3,
    pub dir: Vec3,
    pub pbore: f32,
    pub pconnect: String,
}


pub mod geo_params_data {
    use serde_derive::{Deserialize, Serialize};
    #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
    pub enum CateGeoParam {
        Boxi(super::CateBoxImpliedParam),
        Box(super::CateBoxParam),
        Cone(super::CateConeParam),
        LCylinder(super::CateLCylinderParam),
        SCylinder(super::CateSCylinderParam),

        Dish(super::CateDishParam),
        Extrusion(super::CateExtrusionParam),
        Profile(super::CateProfileParam),
        Line(super::CateLineParam),
        Pyramid(super::CatePyramidParam),
        RectTorus(super::CateRectTorusParam),
        Revolution(super::CateRevolutionParam),
        Sline(super::CateSplineParam),
        SlopeBottomCylinder(super::CateSlopeBottomCylinderParam),
        Snout(super::CateSnoutParam),
        Sphere(super::CateSphereParam),
        Torus(super::CateTorusParam),
        TubeImplied(super::CateTubeImpliedParam),
        SVER(super::CateSverParam),
        Unknown,
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize,  Debug)]
pub struct CateBoxImpliedParam {
    pub axis: Option<CateAxisParam>,
    pub x_length: f32,
    pub z_length: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateBoxParam {
    pub size: Vec<f32>,
    pub offset: Vec<f32>,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateConeParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSCylinderParam {
    pub refno: RefU64,
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub height: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize,Debug)]
pub struct CateLCylinderParam {
    pub refno: RefU64,
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub dist_to_top: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

///拉伸的基本体
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateExtrusionParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub height: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub verts: Vec<Vec3>,  //2D points
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
    pub frads: Vec<f32>,
}

//structural annulus
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SannData {
    pub xy: [f32; 2],
    pub dxy: [f32; 2],
    pub ptaxis: Option<CateAxisParam>,
    pub pangle: f32,
    pub pradius: f32,
    pub pwidth: f32,
    pub drad: f32,
    pub dwid: f32,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SProfileData {
    pub verts: Vec<Vec3>,
    pub frads: Vec<f32>,
    pub normal_axis: Vec3,
    pub plin_pos: Vec2,
    pub plin_axis: Vec3,
}

//截面的处理，还需要旋转自身的平面
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum CateProfileParam{
    SPRO(SProfileData),
    SANN(SannData),
    None,
}


#[derive(Clone, PartialEq, Serialize, Deserialize,Debug)]
pub struct CateDishParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub height: f32,
    pub diameter: f32,
    pub radius: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize,Debug)]
pub struct CateLineParam {
    
    pub pa: ::core::option::Option<CateAxisParam>,
    
    pub pb: ::core::option::Option<CateAxisParam>,
    
    pub diameter: f64,
    
    pub centre_line_flag: bool,
    
    pub tube_flag: bool,
    pub refno: RefU64,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CatePyramidParam {
    pub refno: RefU64,
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub pc: Option<CateAxisParam>,
    pub x_bottom: f32,
    pub y_bottom: f32,
    pub x_top: f32,
    pub y_top: f32,
    pub dist_to_btm: f32,
    pub dist_to_top: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}
/// 截面为矩形的弯管
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateRectTorusParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub height: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateRevolutionParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub angle: f32,
    pub verts: Vec<Vec3>,
    pub frads: Vec<f32>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSplineParam {
    pub start_pt: Vec<f32>,
    pub end_pt: Vec<f32>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSlopeBottomCylinderParam {
    pub axis: Option<CateAxisParam>,
    pub height: f32,
    pub diameter: f32,
    pub dist_to_btm: f32,
    pub x_shear: f32,
    pub y_shear: f32,
    pub alt_x_shear: f32,
    pub alt_y_shear: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}
/// 圆台 或 管嘴
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSnoutParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub dist_to_top: f32,
    pub btm_diameter: f32,
    pub top_diameter: f32,
    pub offset: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}
/// 球
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSphereParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_center: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}
///元件库里的torus参数
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateTorusParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateTubeImpliedParam {
    pub axis: Option<CateAxisParam>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CateSverParam{
    
    pub x: f64,
    
    pub y: f64,
    
    pub radius:f64,
}



