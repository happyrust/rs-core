use std::collections::BTreeMap;
use std::future::Future;
use dashmap::DashMap;
use glam::{Vec2, Vec3};
use parry2d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use crate::parsed_data::geo_params_data::{CateGeoParam, PdmsGeoParam};
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
    pub components: Vec<CateGeomsInfo>,
}

///元件库的集合信息
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CateGeomsInfo {
    //catref
    pub refno: RefU64,
    pub geometries: Vec<CateGeoParam>,
    /// 和dsign发生运算的负实体数据
    pub n_geometries: Vec<CateGeoParam>,
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
    pub type_name: String,
    pub radius: f32,
    //desi 里的radius
    pub angle: f32,
    //desi 里的angle
    ///desi 里的height
    pub height: f32,
    pub pwid: f32,
    pub pang: f32,
    //元件库里的angle
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
    pub paxises: Vec<Option<CateAxisParam>>,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    /// Fillet radius
    pub frads: Vec<f32>,
    pub prad: f32,

    /// plin 数据
    pub plin_pos: Vec2,
    pub plin_plax: Vec3,
}


#[derive(Clone, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Serialize, Deserialize, Debug, Default)]
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
    use crate::prim_geo::ctorus::CTorus;
    use crate::prim_geo::cylinder::*;
    use crate::prim_geo::dish::Dish;
    use crate::prim_geo::extrusion::Extrusion;
    use crate::prim_geo::lpyramid::LPyramid;
    use crate::prim_geo::polyhedron::Polyhedron;
    use crate::prim_geo::pyramid::Pyramid;
    use crate::prim_geo::revolution::Revolution;
    use crate::prim_geo::rtorus::RTorus;
    use crate::prim_geo::sbox::SBox;
    use crate::prim_geo::snout::LSnout;
    use crate::prim_geo::sphere::Sphere;
    use crate::rvm_types::RvmShapeTypeData;


    #[derive(Clone, Serialize, Deserialize, Debug, Default)]
    pub enum CateGeoParam {
        #[default]
        Unknown,
        //元件库的几何体几何存储
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
    }


    #[derive(Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Serialize, Deserialize, Debug, Default)]
    pub enum PdmsGeoParam {
        #[default]
        Unknown,
        //基本体的几何体存储
        PrimBox(SBox),
        PrimLSnout(LSnout),
        PrimDish(Dish),
        PrimSphere(Sphere),
        PrimCTorus(CTorus),
        PrimRTorus(RTorus),
        PrimPyramid(Pyramid),
        PrimSCylinder(SCylinder),
        PrimLCylinder(LCylinder),
        PrimRevolution(Revolution),
        PrimExtrusion(Extrusion),
        PrimPolyhedron(Polyhedron),
        CompoundShape,
    }

    impl PdmsGeoParam {
        pub fn into_rvm_pri_num(&self) -> Option<u8> {
            match self {
                PdmsGeoParam::Unknown => { None }
                PdmsGeoParam::CompoundShape => { None }
                PdmsGeoParam::PrimBox(_) => { Some(2) }
                PdmsGeoParam::PrimLSnout(_) => { Some(7) }
                PdmsGeoParam::PrimDish(_) => { Some(6) }
                PdmsGeoParam::PrimSphere(_) => { Some(9) }
                PdmsGeoParam::PrimCTorus(_) => { Some(4) }
                PdmsGeoParam::PrimRTorus(_) => { Some(3) }
                PdmsGeoParam::PrimPyramid(_) => { Some(1) }
                PdmsGeoParam::PrimSCylinder(_) => { Some(8) }
                PdmsGeoParam::PrimLCylinder(_) => { Some(8) }
                PdmsGeoParam::PrimRevolution(_) => { Some(10) }
                PdmsGeoParam::PrimExtrusion(_) => { Some(11) }
                PdmsGeoParam::PrimPolyhedron(_) => { Some(12) }
            }
        }

        pub fn convert_rvm_pri_data(&self) -> Vec<u8> {
            match &self {
                PdmsGeoParam::PrimBox(data) => {
                    RvmShapeTypeData::Box([data.size.x, data.size.y, data.size.z]).convert_shape_type_to_bytes()
                }
                PdmsGeoParam::PrimLSnout(data) => {
                    let height = (data.ptdi - data.pbdi).abs();
                    let bottom_radius = data.pbdm / 2.0;
                    let top_radius = data.ptdm / 2.0;
                    let offset = data.poff;
                    RvmShapeTypeData::Snout([bottom_radius, top_radius, height, offset, 0.0, 0.0, 0.0, 0.0, 0.0]).convert_shape_type_to_bytes()
                }
                PdmsGeoParam::PrimDish(data) => {
                    let radius = data.pdia / 2.0;
                    let height = data.pheig;
                    RvmShapeTypeData::SphericalDish([radius, height]).convert_shape_type_to_bytes()
                }
                PdmsGeoParam::PrimCTorus(data) => {
                    let in_torus = (data.rout - data.rins) / 2.0;
                    let out_torus = data.rout - in_torus;
                    let angle = (data.angle / 180.0) * std::f32::consts::PI;
                    RvmShapeTypeData::CircularTorus([out_torus, in_torus, angle]).convert_shape_type_to_bytes()
                }
                PdmsGeoParam::PrimRTorus(data) => {
                    let out_torus = data.rout;
                    let len = data.rout - data.rins;
                    let height = data.height;
                    let angle = (data.angle / 180.0) * std::f32::consts::PI;
                    RvmShapeTypeData::RectangularTorus([out_torus, len, height, angle]).convert_shape_type_to_bytes()
                }
                PdmsGeoParam::PrimPyramid(data) => {
                    let bottom_width = data.pbbt;
                    let bottom_length = data.pbbt;
                    let top_width = data.pctp;
                    let top_length = data.pcbt;
                    let x_offset = data.pbof;
                    let y_offset = data.pcof;
                    let height = (data.pbdi - data.ptdi).abs();
                    RvmShapeTypeData::Pyramid([data.pbbt, data.pcbt, data.pbtp, data.pctp, x_offset, y_offset, height]).convert_shape_type_to_bytes()
                }
                PdmsGeoParam::PrimSCylinder(data) => {
                    let radius = data.pdia / 2.0;
                    RvmShapeTypeData::Cylinder([radius, data.phei]).convert_shape_type_to_bytes()
                }
                _ => { vec![] }
            }
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateBoxImpliedParam {
    pub axis: Option<CateAxisParam>,
    pub x_length: f32,
    pub z_length: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateBoxParam {
    pub size: Vec3,
    pub offset: Vec3,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateConeParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateSCylinderParam {
    pub refno: RefU64,
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub height: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateExtrusionParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub height: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub verts: Vec<Vec3>,
    //2D points
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
    pub frads: Vec<f32>,
}

//structural annulus
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct SannData {
    pub xy: Vec2,
    pub dxy: Vec2,
    pub paxis: Option<CateAxisParam>,
    pub pangle: f32,
    pub pradius: f32,
    pub pwidth: f32,
    pub drad: f32,
    pub dwid: f32,

    pub plin_pos: Vec2,
    pub plin_axis: Vec3,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct SProfileData {
    pub verts: Vec<Vec3>,
    pub frads: Vec<f32>,
    pub normal_axis: Vec3,
    pub plin_pos: Vec2,
    pub plin_axis: Vec3,
}

//截面的处理，还需要旋转自身的平面
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum CateProfileParam {
    #[default]
    UNKOWN,
    SPRO(SProfileData),
    SANN(SannData),
}

impl CateProfileParam {
    pub fn get_bbox(&self) -> Option<Aabb> {
        match self {
            Self::UNKOWN => None,
            Self::SANN(s) => {
                Some(Aabb::new(nalgebra::Vector2::from(s.xy + s.dxy - Vec2::ONE * s.drad).into(),
                               nalgebra::Vector2::from(s.xy + s.dxy + Vec2::ONE * s.drad).into()))
            }
            Self::SPRO(s) => {
                let pts = s.verts.iter().map(|x|
                    nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate()))
                ).collect::<Vec<_>>();
                Some(Aabb::from_points(&pts))
            }
        }
    }
}


#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateLineParam {
    pub pa: ::core::option::Option<CateAxisParam>,

    pub pb: ::core::option::Option<CateAxisParam>,

    pub diameter: f64,

    pub centre_line_flag: bool,

    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateRectTorusParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub height: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateSplineParam {
    pub start_pt: Vec<f32>,
    pub end_pt: Vec<f32>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateSphereParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_center: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

///元件库里的torus参数
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateTorusParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateTubeImpliedParam {
    pub axis: Option<CateAxisParam>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CateSverParam {
    pub x: f64,

    pub y: f64,

    pub radius: f64,
}



