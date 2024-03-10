use std::collections::BTreeMap;

use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::types::*;
use glam::{Vec2, Vec3};
use parry2d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};

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
    pub dxy: Vec<Vec2>,
    pub drad: f32,
    pub dwid: f32,
    pub lengths: Vec<f32>,
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

///需要存储到数据库中
#[derive(
    Clone,
    PartialEq,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Debug,
)]
pub struct CateAxisParam {
    pub refno: RefU64,
    pub number: i32,
    pub pt: Vec3,
    pub dir: Vec3,
    pub ref_dir: Vec3,
    pub pbore: f32,
    pub pwidth: f32,
    pub pheight: f32,
    pub pconnect: String,
}

impl Default for CateAxisParam {
    fn default() -> Self{
        Self{
            refno: Default::default(),
            number: 0,
            pt: Default::default(),
            dir: Vec3::Y,
            ref_dir: Default::default(),
            pbore: 0.0,
            pwidth: 0.0,
            pheight: 0.0,
            pconnect: "".to_string(),
        }
    }
}

pub mod geo_params_data {
    use crate::prim_geo::ctorus::CTorus;
    use crate::prim_geo::{cylinder::*, LPyramid};
    use crate::prim_geo::dish::Dish;
    use crate::prim_geo::extrusion::Extrusion;
    #[cfg(feature = "opencascade_rs")]
    use opencascade::primitives::*;
    use serde_derive::{Deserialize, Serialize};

    use crate::prim_geo::polyhedron::Polyhedron;
    use crate::prim_geo::pyramid::Pyramid;
    use crate::prim_geo::revolution::Revolution;
    use crate::prim_geo::rtorus::RTorus;
    use crate::prim_geo::sbox::SBox;
    use crate::prim_geo::snout::LSnout;
    use crate::prim_geo::sphere::Sphere;
    use crate::prim_geo::sweep_solid::SweepSolid;
    use crate::rvm_types::RvmShapeTypeData;
    use crate::shape::pdms_shape::{BrepShapeTrait, RsVec3};

    #[derive(Clone, Serialize, Deserialize, Debug, Default)]
    pub enum CateGeoParam {
        #[default]
        Unknown,
        //元件库的几何体几何存储
        Box(super::CateBoxParam),
        Cone(super::CateSnoutParam),
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
        BoxImplied(super::CateBoxImpliedParam),
        SVER(super::CateSverParam),
    }

    #[derive(
        Clone,
        rkyv::Archive,
        rkyv::Deserialize,
        rkyv::Serialize,
        Serialize,
        Deserialize,
        Debug,
        Default,
    )]
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
        PrimLPyramid(LPyramid),
        PrimSCylinder(SCylinder),
        PrimLCylinder(LCylinder),
        PrimRevolution(Revolution),
        PrimExtrusion(Extrusion),
        PrimPolyhedron(Polyhedron),
        PrimLoft(SweepSolid),
        CompoundShape,
    }

    impl PdmsGeoParam {
        ///获得关键点
        pub fn key_points(&self) -> Vec<RsVec3> {
            match self {
                PdmsGeoParam::Unknown => vec![],
                PdmsGeoParam::PrimBox(s) => s.key_points(),
                PdmsGeoParam::PrimLSnout(s) => s.key_points(),
                PdmsGeoParam::PrimDish(s) => s.key_points(),
                PdmsGeoParam::PrimSphere(s) => s.key_points(),
                PdmsGeoParam::PrimCTorus(s) => s.key_points(),
                PdmsGeoParam::PrimRTorus(s) => s.key_points(),
                PdmsGeoParam::PrimPyramid(s) => s.key_points(),
                PdmsGeoParam::PrimLPyramid(s) => s.key_points(),
                PdmsGeoParam::PrimSCylinder(s) => s.key_points(),
                PdmsGeoParam::PrimLCylinder(s) => s.key_points(),
                PdmsGeoParam::PrimRevolution(s) => s.key_points(),
                PdmsGeoParam::PrimExtrusion(s) => s.key_points(),
                PdmsGeoParam::PrimPolyhedron(s) => s.key_points(),
                PdmsGeoParam::PrimLoft(s) => s.key_points(),
                PdmsGeoParam::CompoundShape => vec![],
            }
        }

        #[cfg(feature = "opencascade_rs")]
        pub fn gen_occ_shape(&self) -> Option<Shape> {
            match self {
                PdmsGeoParam::PrimSCylinder(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimLCylinder(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimExtrusion(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimLoft(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::Unknown => None,
                PdmsGeoParam::PrimBox(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimLSnout(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimDish(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimSphere(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimCTorus(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimRTorus(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimPyramid(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimLPyramid(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimRevolution(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::PrimPolyhedron(s) => s.gen_occ_shape().ok(),
                PdmsGeoParam::CompoundShape => None,
            }
        }

        pub fn into_rvm_pri_num(&self) -> Option<u8> {
            match self {
                // PdmsGeoParam::Unknown => None,
                PdmsGeoParam::CompoundShape => None,
                PdmsGeoParam::PrimBox(_) => Some(2),
                PdmsGeoParam::PrimLSnout(_) => Some(7),
                PdmsGeoParam::PrimDish(_) => Some(6),
                PdmsGeoParam::PrimSphere(_) => Some(9),
                PdmsGeoParam::PrimCTorus(_) => Some(4),
                PdmsGeoParam::PrimRTorus(_) => Some(3),
                PdmsGeoParam::PrimPyramid(_) => Some(1),
                PdmsGeoParam::PrimSCylinder(_) => Some(8),
                PdmsGeoParam::PrimLCylinder(_) => Some(8),
                PdmsGeoParam::PrimRevolution(_) => Some(10),
                PdmsGeoParam::PrimExtrusion(_) => Some(11),
                PdmsGeoParam::PrimPolyhedron(_) => Some(12),
                // PdmsGeoParam::PrimLoft(_) => None,
                _ => None,
            }
        }

        pub fn convert_rvm_pri_data(&self) -> Option<Vec<u8>> {
            match &self {
                PdmsGeoParam::PrimBox(data) => Some(
                    RvmShapeTypeData::Box([data.size.x, data.size.y, data.size.z])
                        .convert_shape_type_to_bytes(),
                ),
                PdmsGeoParam::PrimLSnout(data) => {
                    let height = (data.ptdi - data.pbdi).abs();
                    let bottom_radius = data.pbdm / 2.0;
                    let top_radius = data.ptdm / 2.0;
                    let offset = data.poff;
                    Some(
                        RvmShapeTypeData::Snout([
                            bottom_radius,
                            top_radius,
                            height,
                            offset,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                        ])
                        .convert_shape_type_to_bytes(),
                    )
                }
                PdmsGeoParam::PrimDish(data) => {
                    let radius = data.pdia / 2.0;
                    let height = data.pheig;
                    Some(
                        RvmShapeTypeData::SphericalDish([radius, height])
                            .convert_shape_type_to_bytes(),
                    )
                }
                PdmsGeoParam::PrimCTorus(data) => {
                    let in_torus = (data.rout - data.rins) / 2.0;
                    let out_torus = data.rout - in_torus;
                    let angle = (data.angle / 180.0) * std::f32::consts::PI;
                    Some(
                        RvmShapeTypeData::CircularTorus([out_torus, in_torus, angle])
                            .convert_shape_type_to_bytes(),
                    )
                }
                PdmsGeoParam::PrimRTorus(data) => {
                    let out_torus = data.rout;
                    let len = data.rout - data.rins;
                    let height = data.height;
                    let angle = (data.angle / 180.0) * std::f32::consts::PI;
                    Some(
                        RvmShapeTypeData::RectangularTorus([out_torus, len, height, angle])
                            .convert_shape_type_to_bytes(),
                    )
                }
                PdmsGeoParam::PrimPyramid(data) => {
                    let _bottom_width = data.pbbt;
                    let _bottom_length = data.pbbt;
                    let _top_width = data.pctp;
                    let _top_length = data.pcbt;
                    let x_offset = data.pbof;
                    let y_offset = data.pcof;
                    let height = (data.pbdi - data.ptdi).abs();
                    Some(
                        RvmShapeTypeData::Pyramid([
                            data.pbbt, data.pcbt, data.pbtp, data.pctp, x_offset, y_offset, height,
                        ])
                        .convert_shape_type_to_bytes(),
                    )
                }
                PdmsGeoParam::PrimSCylinder(data) => {
                    let radius = data.pdia / 2.0;
                    Some(
                        RvmShapeTypeData::Cylinder([radius, data.phei])
                            .convert_shape_type_to_bytes(),
                    )
                }
                _ => None,
            }
        }
    }
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateBoxParam {
    pub size: Vec3,
    pub offset: Vec3,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateConeParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateSCylinderParam {
    pub refno: RefU64,
    pub axis: Option<CateAxisParam>,
    pub dist_to_btm: f32,
    pub height: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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

///一般的由顶点组成的截面信息
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct SProfileData {
    pub verts: Vec<Vec2>,
    pub frads: Vec<f32>,
    pub normal_axis: Vec3,
    pub plin_pos: Vec2,
    pub plin_axis: Vec3,
}

///一般的由顶点组成的截面信息
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct SRectData {
    pub center: Vec2,
    pub size: Vec2,
    pub dxy: Vec2,
    pub normal_axis: Vec3,
    pub plin_pos: Vec2,
    pub plin_axis: Vec3,
}

impl SRectData{
    pub fn convert_to_spro(&self) -> SProfileData{
        let c = self.center + self.dxy;
        let h = self.size/2.0;
        SProfileData{
            verts: vec![c - h, c + Vec2::new(h.x, -h.y), c + h, c + Vec2::new(-h.x, h.y) ],
            frads: vec![0.0; 4],
            normal_axis: self.normal_axis,
            plin_pos: self.plin_pos,
            plin_axis: self.plin_axis,
        }
    }
}

//截面的处理，还需要旋转自身的平面
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum CateProfileParam {
    #[default]
    UNKOWN,
    SPRO(SProfileData),
    SANN(SannData),
    SREC(SRectData),
}

impl CateProfileParam {

    pub fn get_plax(&self) -> Vec3{
        match self {
            CateProfileParam::UNKOWN => Vec3::Y,
            CateProfileParam::SPRO(s) => s.normal_axis.normalize(),
            CateProfileParam::SANN(s) => s.paxis.as_ref().map(|x| x.dir).unwrap_or(Vec3::Y),
            CateProfileParam::SREC(s) => s.normal_axis.normalize(),
        }
    }

    pub fn get_bbox(&self) -> Option<Aabb> {
        match self {
            CateProfileParam::UNKOWN => None,
            CateProfileParam::SANN(s) => Some(Aabb::new(
                nalgebra::Point2::from(s.xy + s.dxy - Vec2::ONE * s.drad),
                nalgebra::Point2::from(s.xy + s.dxy + Vec2::ONE * s.drad),
            )),
            CateProfileParam::SPRO(s) => {
                let pts = s
                    .verts
                    .iter()
                    .map(|x| nalgebra::Point2::from(*x))
                    .collect::<Vec<_>>();
                Some(Aabb::from_points(&pts))
            }
            CateProfileParam::SREC(s) => Some(Aabb::new(
                nalgebra::Point2::from(s.center + s.dxy - s.size/2.0),
                nalgebra::Point2::from(s.center + s.dxy + s.size/2.0),
            )),
        }
    }
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateLineParam {
    pub pa: ::core::option::Option<CateAxisParam>,

    pub pb: ::core::option::Option<CateAxisParam>,

    pub diameter: f64,

    pub centre_line_flag: bool,

    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateRectTorusParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub height: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateSplineParam {
    pub start_pt: Vec<f32>,
    pub end_pt: Vec<f32>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
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
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateSphereParam {
    pub axis: Option<CateAxisParam>,
    pub dist_to_center: f32,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

///元件库里的torus参数
#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateTorusParam {
    pub pa: Option<CateAxisParam>,
    pub pb: Option<CateAxisParam>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
    pub refno: RefU64,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateTubeImpliedParam {
    pub axis: Option<CateAxisParam>,
    pub diameter: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateBoxImpliedParam {
    pub axis: Option<CateAxisParam>,
    pub width: f32,
    pub height: f32,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CateSverParam {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}
